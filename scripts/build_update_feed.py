#!/usr/bin/env python3
"""Build and verify Reyn Studio's canonical, Ed25519-signed update feed."""

from __future__ import annotations

import argparse
import base64
import binascii
import hashlib
import json
import os
import re
import sys
import tempfile
from pathlib import Path
from typing import Any, Mapping, Sequence
from urllib.parse import urlsplit

try:
    from cryptography.exceptions import InvalidSignature
    from cryptography.hazmat.primitives.asymmetric.ed25519 import (
        Ed25519PrivateKey,
        Ed25519PublicKey,
    )
except ImportError as error:  # pragma: no cover - exercised only without dependency
    InvalidSignature = None  # type: ignore[assignment]
    Ed25519PrivateKey = None  # type: ignore[assignment,misc]
    Ed25519PublicKey = None  # type: ignore[assignment,misc]
    _CRYPTOGRAPHY_IMPORT_ERROR: ImportError | None = error
else:
    _CRYPTOGRAPHY_IMPORT_ERROR = None


FEED_SCHEMA = "com.reyn.studio.update-feed/1"
SIGNATURE_SCHEMA = "com.reyn.studio.update-signature/1"
ALGORITHM = "Ed25519"
DEFAULT_ALLOWED_HOSTS = ("reynflow.com",)
PLATFORMS = ("macos-arm64", "windows-x64")
SHA256_RE = re.compile(r"[0-9a-f]{64}\Z")
VERSION_RE = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")
KEY_ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}\Z")

FEED_FIELDS = {
    "schema",
    "version",
    "release_sequence",
    "published",
    "expires",
    "minimum_updater_version",
    "channel",
    "changelog_url",
    "key_id",
    "artifacts",
}
ARTIFACT_FIELDS = {
    "platform",
    "architecture",
    "minimum_os",
    "url",
    "archive_name",
    "bytes",
    "sha256",
    "developer_id_signed",
    "notarized",
    "authenticode_signed",
}
SIGNATURE_FIELDS = {"schema", "key_id", "algorithm", "signature"}


def _require_cryptography() -> None:
    if _CRYPTOGRAPHY_IMPORT_ERROR is not None:
        raise RuntimeError(
            "the 'cryptography' package is required for Ed25519 signing and verification"
        ) from _CRYPTOGRAPHY_IMPORT_ERROR


def canonical_json_bytes(document: Mapping[str, Any]) -> bytes:
    """Return the one accepted JSON encoding: sorted UTF-8, no whitespace or NaN."""
    return json.dumps(
        document,
        ensure_ascii=False,
        allow_nan=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def _strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field: {key}")
        result[key] = value
    return result


def parse_canonical_json(raw: bytes, *, description: str) -> dict[str, Any]:
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError(f"{description} is not valid UTF-8") from error
    try:
        document = json.loads(
            text,
            object_pairs_hook=_strict_object,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ValueError(f"invalid JSON number: {value}")
            ),
        )
    except (json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"{description} is not valid strict JSON: {error}") from error
    if not isinstance(document, dict):
        raise ValueError(f"{description} must be a JSON object")
    if canonical_json_bytes(document) != raw:
        raise ValueError(f"{description} is not canonical JSON")
    return document


def _require_exact_fields(
    document: Mapping[str, Any], expected: set[str], description: str
) -> None:
    actual = set(document)
    unknown = actual - expected
    missing = expected - actual
    if unknown:
        raise ValueError(f"{description} has unknown fields: {', '.join(sorted(unknown))}")
    if missing:
        raise ValueError(f"{description} is missing fields: {', '.join(sorted(missing))}")


def _require_string(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{field} must be a non-empty string")
    return value


def _require_bool(value: Any, field: str) -> bool:
    if type(value) is not bool:
        raise ValueError(f"{field} must be a boolean")
    return value


def _require_positive_int(value: Any, field: str) -> int:
    if type(value) is not int or value <= 0:
        raise ValueError(f"{field} must be a positive integer")
    return value


def _validate_version(value: Any, field: str) -> str:
    version = _require_string(value, field)
    if VERSION_RE.fullmatch(version) is None:
        raise ValueError(f"{field} must be a canonical major.minor.patch version")
    return version


def _validate_url(value: Any, field: str, allowed_hosts: Sequence[str]) -> str:
    url = _require_string(value, field)
    parts = urlsplit(url)
    normalized_hosts = {host.lower() for host in allowed_hosts if host}
    if (
        parts.scheme != "https"
        or not parts.hostname
        or parts.hostname.lower() not in normalized_hosts
        or parts.username is not None
        or parts.password is not None
        or parts.port not in (None, 443)
        or not parts.path.startswith("/")
        or parts.query
        or parts.fragment
    ):
        raise ValueError(
            f"{field} must be a query-free HTTPS URL on an approved host "
            f"({', '.join(sorted(normalized_hosts))})"
        )
    return url


def _expected_archive_pattern(platform: str, version: str) -> re.Pattern[str]:
    escaped = re.escape(version)
    if platform == "macos-arm64":
        return re.compile(
            rf"Reyn-Studio-{escaped}-build\.[1-9][0-9]*-arm64\.app\.zip\Z"
        )
    return re.compile(rf"Reyn-Studio-{escaped}-windows-x64\.zip\Z")


def validate_feed(
    document: Mapping[str, Any],
    *,
    allowed_hosts: Sequence[str] = DEFAULT_ALLOWED_HOSTS,
) -> None:
    _require_exact_fields(document, FEED_FIELDS, "update feed")
    if document["schema"] != FEED_SCHEMA:
        raise ValueError(f"update feed schema must be {FEED_SCHEMA}")
    version = _validate_version(document["version"], "version")
    _require_positive_int(document["release_sequence"], "release_sequence")
    published = _require_positive_int(document["published"], "published")
    expires = _require_positive_int(document["expires"], "expires")
    if expires <= published:
        raise ValueError("expires must be greater than published")
    _validate_version(document["minimum_updater_version"], "minimum_updater_version")
    channel = _require_string(document["channel"], "channel")
    if re.fullmatch(r"[a-z0-9][a-z0-9-]{0,31}", channel) is None:
        raise ValueError("channel must be a lowercase channel identifier")
    _validate_url(document["changelog_url"], "changelog_url", allowed_hosts)
    key_id = _require_string(document["key_id"], "key_id")
    if KEY_ID_RE.fullmatch(key_id) is None:
        raise ValueError("key_id contains invalid characters")

    artifacts = document["artifacts"]
    if not isinstance(artifacts, list) or len(artifacts) != 2:
        raise ValueError("artifacts must contain exactly two records")
    seen: set[str] = set()
    for index, artifact in enumerate(artifacts):
        description = f"artifact {index}"
        if not isinstance(artifact, dict):
            raise ValueError(f"{description} must be an object")
        _require_exact_fields(artifact, ARTIFACT_FIELDS, description)
        platform = _require_string(artifact["platform"], f"{description}.platform")
        if platform not in PLATFORMS or platform in seen:
            raise ValueError("artifacts must contain exactly macos-arm64 and windows-x64")
        seen.add(platform)
        expected_architecture = "arm64" if platform == "macos-arm64" else "x64"
        if artifact["architecture"] != expected_architecture:
            raise ValueError(
                f"{description}.architecture must be {expected_architecture}"
            )
        _require_string(artifact["minimum_os"], f"{description}.minimum_os")
        archive_name = _require_string(
            artifact["archive_name"], f"{description}.archive_name"
        )
        if Path(archive_name).name != archive_name:
            raise ValueError(f"{description}.archive_name must be a filename")
        if _expected_archive_pattern(platform, version).fullmatch(archive_name) is None:
            raise ValueError(
                f"{description}.archive_name is invalid for {version} {platform}"
            )
        url = _validate_url(artifact["url"], f"{description}.url", allowed_hosts)
        if urlsplit(url).path.rsplit("/", 1)[-1] != archive_name:
            raise ValueError(f"{description}.url must end with archive_name")
        _require_positive_int(artifact["bytes"], f"{description}.bytes")
        digest = _require_string(artifact["sha256"], f"{description}.sha256")
        if SHA256_RE.fullmatch(digest) is None:
            raise ValueError(f"{description}.sha256 must be lowercase SHA-256 hex")
        for field in (
            "developer_id_signed",
            "notarized",
            "authenticode_signed",
        ):
            _require_bool(artifact[field], f"{description}.{field}")
        if platform == "macos-arm64":
            if artifact["authenticode_signed"]:
                raise ValueError("macOS artifact cannot be Authenticode signed")
        elif artifact["developer_id_signed"] or artifact["notarized"]:
            raise ValueError("Windows artifact cannot be Developer ID signed or notarized")
    if seen != set(PLATFORMS):
        raise ValueError("artifacts must contain exactly macos-arm64 and windows-x64")


def validate_release_progression(
    document: Mapping[str, Any],
    previous: Mapping[str, Any],
    *,
    allowed_hosts: Sequence[str] = DEFAULT_ALLOWED_HOSTS,
) -> None:
    """Reject sequence or publication-time rollback against a prior feed."""
    validate_feed(document, allowed_hosts=allowed_hosts)
    validate_feed(previous, allowed_hosts=allowed_hosts)
    if document["release_sequence"] <= previous["release_sequence"]:
        raise ValueError(
            "release_sequence must be greater than the previous feed sequence"
        )
    if document["published"] <= previous["published"]:
        raise ValueError("published must be greater than the previous feed timestamp")


def validate_signature_document(document: Mapping[str, Any], *, key_id: str) -> bytes:
    _require_exact_fields(document, SIGNATURE_FIELDS, "signature document")
    if document["schema"] != SIGNATURE_SCHEMA:
        raise ValueError(f"signature schema must be {SIGNATURE_SCHEMA}")
    if document["key_id"] != key_id:
        raise ValueError("signature key_id does not match feed key_id")
    if document["algorithm"] != ALGORITHM:
        raise ValueError(f"signature algorithm must be {ALGORITHM}")
    return _decode_base64(document["signature"], "signature", expected_bytes=64)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def artifact_record(
    *,
    platform: str,
    path: Path,
    minimum_os: str,
    url: str,
    developer_id_signed: bool = False,
    notarized: bool = False,
    authenticode_signed: bool = False,
) -> dict[str, Any]:
    if not path.is_file():
        raise ValueError(f"package file does not exist: {path}")
    return {
        "platform": platform,
        "architecture": "arm64" if platform == "macos-arm64" else "x64",
        "minimum_os": minimum_os,
        "url": url,
        "archive_name": path.name,
        "bytes": path.stat().st_size,
        "sha256": sha256_file(path),
        "developer_id_signed": developer_id_signed,
        "notarized": notarized,
        "authenticode_signed": authenticode_signed,
    }


def build_feed(
    *,
    version: str,
    release_sequence: int,
    published: int,
    expires: int,
    minimum_updater_version: str,
    channel: str,
    changelog_url: str,
    key_id: str,
    macos_path: Path,
    macos_minimum_os: str,
    macos_url: str,
    windows_path: Path,
    windows_minimum_os: str,
    windows_url: str,
    developer_id_signed: bool,
    notarized: bool,
    authenticode_signed: bool,
    allowed_hosts: Sequence[str] = DEFAULT_ALLOWED_HOSTS,
) -> dict[str, Any]:
    document = {
        "schema": FEED_SCHEMA,
        "version": version,
        "release_sequence": release_sequence,
        "published": published,
        "expires": expires,
        "minimum_updater_version": minimum_updater_version,
        "channel": channel,
        "changelog_url": changelog_url,
        "key_id": key_id,
        "artifacts": [
            artifact_record(
                platform="macos-arm64",
                path=macos_path,
                minimum_os=macos_minimum_os,
                url=macos_url,
                developer_id_signed=developer_id_signed,
                notarized=notarized,
            ),
            artifact_record(
                platform="windows-x64",
                path=windows_path,
                minimum_os=windows_minimum_os,
                url=windows_url,
                authenticode_signed=authenticode_signed,
            ),
        ],
    }
    validate_feed(document, allowed_hosts=allowed_hosts)
    return document


def _decode_base64(value: Any, field: str, *, expected_bytes: int) -> bytes:
    text = _require_string(value, field)
    try:
        decoded = base64.b64decode(text, validate=True)
    except (binascii.Error, ValueError) as error:
        raise ValueError(f"{field} must be valid standard base64") from error
    if len(decoded) != expected_bytes:
        raise ValueError(f"{field} must decode to exactly {expected_bytes} bytes")
    if base64.b64encode(decoded).decode("ascii") != text:
        raise ValueError(f"{field} must use canonical padded base64")
    return decoded


def load_private_seed(key_file: Path | None, environ: Mapping[str, str] = os.environ) -> bytes:
    env_value = environ.get("REYN_UPDATE_SIGNING_KEY_B64")
    if key_file is not None and env_value:
        raise ValueError(
            "provide the update signing seed through either --key-file or "
            "REYN_UPDATE_SIGNING_KEY_B64, not both"
        )
    if key_file is not None:
        if not key_file.is_file():
            raise ValueError(f"signing key file does not exist: {key_file}")
        raw = key_file.read_bytes()
        if len(raw) == 32:
            return raw
        try:
            return _decode_base64(
                raw.decode("ascii").strip(), "signing key file", expected_bytes=32
            )
        except UnicodeDecodeError as error:
            raise ValueError(
                "signing key file must contain a raw 32-byte seed or its base64 encoding"
            ) from error
    if env_value:
        return _decode_base64(
            env_value, "REYN_UPDATE_SIGNING_KEY_B64", expected_bytes=32
        )
    raise ValueError(
        "missing signing seed: set REYN_UPDATE_SIGNING_KEY_B64 or pass --key-file"
    )


def sign_feed(feed_bytes: bytes, *, key_id: str, private_seed: bytes) -> bytes:
    _require_cryptography()
    if len(private_seed) != 32:
        raise ValueError("Ed25519 private seed must be exactly 32 bytes")
    signature = Ed25519PrivateKey.from_private_bytes(private_seed).sign(feed_bytes)
    document = {
        "schema": SIGNATURE_SCHEMA,
        "key_id": key_id,
        "algorithm": ALGORITHM,
        "signature": base64.b64encode(signature).decode("ascii"),
    }
    return canonical_json_bytes(document)


def verify_feed(
    feed_bytes: bytes,
    signature_bytes: bytes,
    *,
    expected_public_key_b64: str,
    allowed_hosts: Sequence[str] = DEFAULT_ALLOWED_HOSTS,
) -> dict[str, Any]:
    _require_cryptography()
    feed = parse_canonical_json(feed_bytes, description="update feed")
    validate_feed(feed, allowed_hosts=allowed_hosts)
    signature_document = parse_canonical_json(
        signature_bytes, description="signature document"
    )
    signature = validate_signature_document(signature_document, key_id=feed["key_id"])
    public_key = _decode_base64(
        expected_public_key_b64, "expected public key", expected_bytes=32
    )
    try:
        Ed25519PublicKey.from_public_bytes(public_key).verify(signature, feed_bytes)
    except InvalidSignature as error:
        raise ValueError("update feed signature verification failed") from error
    return feed


def atomic_write(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def _allowed_hosts(args: argparse.Namespace) -> tuple[str, ...]:
    hosts = tuple(args.allowed_host or DEFAULT_ALLOWED_HOSTS)
    if not hosts:
        raise ValueError("at least one approved URL host is required")
    return hosts


def _run_build(args: argparse.Namespace) -> int:
    allowed_hosts = _allowed_hosts(args)
    feed = build_feed(
        version=args.version,
        release_sequence=args.release_sequence,
        published=args.published,
        expires=args.expires,
        minimum_updater_version=args.minimum_updater_version,
        channel=args.channel,
        changelog_url=args.changelog_url,
        key_id=args.key_id,
        macos_path=args.macos_package,
        macos_minimum_os=args.macos_minimum_os,
        macos_url=args.macos_url,
        windows_path=args.windows_package,
        windows_minimum_os=args.windows_minimum_os,
        windows_url=args.windows_url,
        developer_id_signed=args.developer_id_signed,
        notarized=args.notarized,
        authenticode_signed=args.authenticode_signed,
        allowed_hosts=allowed_hosts,
    )
    if args.previous_feed is not None:
        previous = parse_canonical_json(
            args.previous_feed.read_bytes(), description="previous update feed"
        )
        validate_release_progression(feed, previous, allowed_hosts=allowed_hosts)
    feed_bytes = canonical_json_bytes(feed)
    signature_bytes = sign_feed(
        feed_bytes,
        key_id=args.key_id,
        private_seed=load_private_seed(args.key_file),
    )
    output_dir = args.output_dir.resolve()
    atomic_write(output_dir / "latest.json", feed_bytes)
    atomic_write(output_dir / "latest.sig", signature_bytes)
    print(f"Wrote canonical update feed: {output_dir / 'latest.json'}")
    print(f"Wrote Ed25519 signature: {output_dir / 'latest.sig'}")
    return 0


def _run_verify(args: argparse.Namespace) -> int:
    verify_feed(
        args.feed.read_bytes(),
        args.signature.read_bytes(),
        expected_public_key_b64=args.public_key_b64,
        allowed_hosts=_allowed_hosts(args),
    )
    print("Update feed signature and schema verified.")
    return 0


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    build = subparsers.add_parser("build", help="build and sign latest.json")
    build.add_argument("--version", required=True)
    build.add_argument("--release-sequence", required=True, type=int)
    build.add_argument("--published", required=True, type=int)
    build.add_argument("--expires", required=True, type=int)
    build.add_argument("--minimum-updater-version", required=True)
    build.add_argument("--channel", required=True)
    build.add_argument("--changelog-url", required=True)
    build.add_argument("--key-id", required=True)
    build.add_argument("--macos-package", required=True, type=Path)
    build.add_argument("--macos-minimum-os", required=True)
    build.add_argument("--macos-url", required=True)
    build.add_argument("--windows-package", required=True, type=Path)
    build.add_argument("--windows-minimum-os", required=True)
    build.add_argument("--windows-url", required=True)
    build.add_argument("--developer-id-signed", action="store_true")
    build.add_argument("--notarized", action="store_true")
    build.add_argument("--authenticode-signed", action="store_true")
    build.add_argument("--key-file", type=Path)
    build.add_argument(
        "--previous-feed",
        type=Path,
        help="canonical prior latest.json used to reject sequence/time rollback",
    )
    build.add_argument("--output-dir", required=True, type=Path)
    build.add_argument("--allowed-host", action="append")
    build.set_defaults(handler=_run_build)

    verify = subparsers.add_parser("verify", help="verify canonical feed and signature")
    verify.add_argument("--feed", required=True, type=Path)
    verify.add_argument("--signature", required=True, type=Path)
    verify.add_argument("--public-key-b64", required=True)
    verify.add_argument("--allowed-host", action="append")
    verify.set_defaults(handler=_run_verify)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    return args.handler(args)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError) as error:
        print(f"update feed failed: {error}", file=sys.stderr)
        raise SystemExit(1)
