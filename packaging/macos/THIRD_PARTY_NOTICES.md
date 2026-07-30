# Reyn Studio macOS Python Runtime Notices

The 0.1.1 macOS package includes Reyn Studio's lightweight Python source closure
and an exact arm64 factory runtime. It does not include model weights, detached
model signatures, a production TUF root, or TUF repository metadata. The
components below are locked in `python-runtime.lock.json`; the runtime also
retains each installed distribution's complete license files.

## cryptography 49.0.0

Project: <https://github.com/pyca/cryptography>

License: Apache License 2.0 OR BSD 3-Clause License.

Copyright (c) individual contributors to the `cryptography` project.

## safetensors 0.8.0

Project: <https://github.com/huggingface/safetensors>

License: Apache License 2.0.

Copyright (c) Hugging Face and contributors.

## python-tuf 7.0.0

Project: <https://github.com/theupdateframework/python-tuf>

License: MIT License.

Copyright (c) The Update Framework contributors.

## securesystemslib 1.4.0

Project: <https://github.com/secure-systems-lab/securesystemslib>

License: MIT License.

Copyright (c) New York University and the securesystemslib contributors.

## NumPy 2.5.1

Project: <https://numpy.org/>

License: BSD 3-Clause License.

Copyright (c) 2005-2026, NumPy Developers.

NumPy wheels contain additional third-party components and notices; those files
remain in the bundled runtime.

## PyTorch 2.13.0

Project: <https://pytorch.org/>

License: BSD 3-Clause License.

Copyright (c) 2016-2026 Facebook, Inc. and PyTorch contributors.

PyTorch distributions include a separate `NOTICE` and licenses for bundled
third-party components; those files remain in the bundled runtime.

## Python 3.14.6

Project: <https://www.python.org/>

License: Python Software Foundation License Version 2 and applicable historical
licenses.

Copyright (c) 2001-2026 Python Software Foundation; all rights reserved.

## Locked transitive runtime distributions

The exact runtime additionally contains:

- cffi 2.1.0 — MIT
- filelock 3.32.2 — Unlicense
- fsspec 2026.7.0 — BSD-3-Clause
- Jinja2 3.1.6 — BSD-3-Clause
- MarkupSafe 3.0.3 — BSD-3-Clause
- mpmath 1.3.0 — BSD-3-Clause
- networkx 3.6.1 — BSD-3-Clause
- pycparser 3.0 — BSD-3-Clause
- setuptools 83.0.0 — MIT
- sympy 1.14.0 — BSD-3-Clause
- typing-extensions 4.16.0 — PSF-2.0
- urllib3 2.7.0 — MIT

The full license texts accompany the upstream distributions in the factory
runtime. This notice is an inventory and attribution index.
