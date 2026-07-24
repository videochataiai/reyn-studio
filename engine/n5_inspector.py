"""Scientific scalar evidence for the N5 Benchmark Cell Inspector.

The inspector receives periodic 2D velocity fields in ``[component, y, x]``
layout on the ``[0, 2*pi)`` domain.  This module derives every displayed map
from those fields in one pass so switching variables in the native UI never
changes the underlying evidence or triggers another model inference.
"""

from __future__ import annotations

import numpy as np


INSPECTOR_VARIABLES = ("velocity", "vorticity", "pressure", "divergence")
INSPECTOR_SIGNED = (False, True, True, True)
INSPECTOR_SCHEMA = "reyn.benchmark-inspector.maps.v2"
INSPECTOR_PROTOCOL_VERSION = 2
INSPECTOR_LAYOUT = "variable,model_reference_error,y,x"
INSPECTOR_DOMAIN = "periodic_2pi"
INSPECTOR_DERIVATIVE = "fourier_spectral_nyquist_zero"
INSPECTOR_PRESSURE = "advective_poisson_density_normalized_zero_mean"
INSPECTOR_UNITS = (
    "solver_velocity_unit",
    "inverse_solver_time_unit",
    "solver_velocity_unit_squared",
    "inverse_solver_time_unit",
)
INSPECTOR_PANEL_SOURCES = (
    ("MODEL", "SOLVER_REFERENCE", "DERIVED"),
    ("DERIVED_FROM_MODEL", "DERIVED_FROM_SOLVER_REFERENCE", "DERIVED"),
    ("RECOVERED_FROM_MODEL", "RECOVERED_FROM_SOLVER_REFERENCE", "DERIVED"),
    ("DERIVED_FROM_MODEL", "DERIVED_FROM_SOLVER_REFERENCE", "DERIVED"),
)


def _velocity(field, name):
    value = np.asarray(field, dtype=np.float64)
    if (
        value.ndim != 3
        or value.shape[0] != 2
        or value.shape[1] != value.shape[2]
        or value.shape[1] < 2
    ):
        raise ValueError(f"{name} must be a square [2,N,N] velocity field")
    if not np.all(np.isfinite(value)):
        raise ValueError(f"{name} contains non-finite values")
    return value


def _wavenumbers(n):
    modes = np.fft.fftfreq(n, d=1.0 / n)
    if n % 2 == 0:
        # A real-valued centered derivative cannot represent the Nyquist mode's
        # sign.  Zeroing it matches Reyn's torch spectral operators.
        modes[n // 2] = 0.0
    return modes.reshape(1, n), modes.reshape(n, 1)


def _kinematics(field):
    """Return ``(vorticity, divergence, pressure)`` for one velocity field."""
    field = _velocity(field, "field")
    n = field.shape[-1]
    kx, ky = _wavenumbers(n)
    u, v = field
    u_hat = np.fft.fft2(u)
    v_hat = np.fft.fft2(v)
    du_dx = np.fft.ifft2(1j * kx * u_hat).real
    du_dy = np.fft.ifft2(1j * ky * u_hat).real
    dv_dx = np.fft.ifft2(1j * kx * v_hat).real
    dv_dy = np.fft.ifft2(1j * ky * v_hat).real

    vorticity = dv_dx - du_dy
    divergence = du_dx + dv_dy

    # Density-normalized recovered pressure:
    #   lap(p) = -div((u.grad)u)
    # with a zero-mean gauge, matching flow_quantities.pressure_from_velocity.
    adv_u = u * du_dx + v * du_dy
    adv_v = u * dv_dx + v * dv_dy
    div_adv_hat = (
        1j * kx * np.fft.fft2(adv_u)
        + 1j * ky * np.fft.fft2(adv_v)
    )
    k2 = kx**2 + ky**2
    safe_k2 = np.where(k2 == 0.0, 1.0, k2)
    pressure_hat = div_adv_hat / safe_k2
    pressure_hat[0, 0] = 0.0
    pressure = np.fft.ifft2(pressure_hat).real
    return vorticity, divergence, pressure


def spatial_divergence(field):
    """Spectral pointwise divergence for a periodic ``[2,N,N]`` field."""
    return _kinematics(field)[1].astype(np.float32)


def recovered_pressure(field):
    """Density-normalized, zero-mean spectral pressure for ``[2,N,N]``."""
    return _kinematics(field)[2].astype(np.float32)


def inspector_variable_maps(prediction, reference):
    """Build model/reference/error maps for every Benchmark Inspector variable.

    Returns ``[variable, panel, y, x]`` maps, where panel order is model,
    solver reference, then derived error.  Velocity's error panel is pointwise
    vector-error magnitude ``|u_model-u_reference|``; signed variables use the
    signed residual ``model-reference``.
    """
    prediction = _velocity(prediction, "prediction")
    reference = _velocity(reference, "reference")
    if prediction.shape != reference.shape:
        raise ValueError("prediction and reference fields must share a shape")

    pred_vort, pred_div, pred_pressure = _kinematics(prediction)
    reference_vort, reference_div, reference_pressure = _kinematics(reference)
    pred_speed = np.sqrt(np.sum(prediction**2, axis=0))
    reference_speed = np.sqrt(np.sum(reference**2, axis=0))
    vector_error = np.sqrt(np.sum((prediction - reference) ** 2, axis=0))

    maps = np.stack(
        [
            np.stack([pred_speed, reference_speed, vector_error]),
            np.stack([pred_vort, reference_vort, pred_vort - reference_vort]),
            np.stack(
                [
                    pred_pressure,
                    reference_pressure,
                    pred_pressure - reference_pressure,
                ]
            ),
            np.stack([pred_div, reference_div, pred_div - reference_div]),
        ]
    )
    return {
        "variables": list(INSPECTOR_VARIABLES),
        "signed": list(INSPECTOR_SIGNED),
        "maps": maps.astype(np.float32),
    }


def inspector_payload(prediction, reference):
    """Encode variable evidence in the sidecar's flat-f32 payload contract."""
    evidence = inspector_variable_maps(prediction, reference)
    maps = evidence["maps"]
    metadata = {
        "schema": INSPECTOR_SCHEMA,
        "protocol_version": INSPECTOR_PROTOCOL_VERSION,
        "shape": list(maps.shape),
        "layout": INSPECTOR_LAYOUT,
        "variables": evidence["variables"],
        "signed": evidence["signed"],
        "units": list(INSPECTOR_UNITS),
        "panel_sources": [list(sources) for sources in INSPECTOR_PANEL_SOURCES],
        "domain": INSPECTOR_DOMAIN,
        "derivative": INSPECTOR_DERIVATIVE,
        "pressure": INSPECTOR_PRESSURE,
    }
    return maps.reshape(-1), metadata
