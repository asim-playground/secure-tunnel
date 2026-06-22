#!/usr/bin/env python3
# Copyright 2026 Asim Ihsan
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

"""SDK release metadata checks and dry-run artifact manifest generation."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import stat
import tarfile
import tomllib
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any


PACKAGE_NAMES = ("swift", "kotlin", "python", "flutter", "go")
GO_MODULE_PATH = "github.com/asim-playground/secure-tunnel/crates/go"


@dataclass(frozen=True)
class Artifact:
    """A generated release artifact recorded in the dry-run manifest."""

    package: str
    path: str
    sha256: str
    size_bytes: int


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def simple_yaml_scalars(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in read_text(path).splitlines():
        if not line or line.startswith((" ", "\t", "#")) or ":" not in line:
            continue
        key, value = line.split(":", 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
    return values


def regex_value(pattern: str, text: str, label: str) -> str:
    match = re.search(pattern, text, flags=re.MULTILINE)
    if match is None:
        raise ValueError(f"missing {label}")
    return match.group(1)


def dependency_version(table: dict[str, Any], name: str) -> str | None:
    value = table.get("dependencies", {}).get(name)
    if isinstance(value, dict):
        version = value.get("version")
        if isinstance(version, str):
            return version
    if isinstance(value, str):
        return value
    return None


def package_metadata(root: Path) -> dict[str, Any]:
    cargo = load_toml(root / "Cargo.toml")
    sdk_cargo = load_toml(root / "crates/sdk/Cargo.toml")
    sdk_ffi_cargo = load_toml(root / "crates/sdk-ffi/Cargo.toml")
    flutter_rust_cargo = load_toml(root / "bindings/flutter/rust/Cargo.toml")
    python_project = load_toml(root / "python/pyproject.toml").get("project", {})
    kotlin_text = read_text(root / "bindings/kotlin/build.gradle.kts")
    flutter_pubspec = simple_yaml_scalars(root / "bindings/flutter/pubspec.yaml")
    swift_text = read_text(root / "bindings/swift/Package.swift")
    go_mod = read_text(root / "crates/go/go.mod")
    go_types = read_text(root / "crates/go/types.go")
    go_binding = read_text(root / "crates/go/binding.go")

    version = cargo["workspace"]["package"]["version"]
    return {
        "schema": "secure-tunnel-sdk-release-metadata-v1",
        "version": version,
        "rust": {
            "workspace_version": version,
            "sdk_dependency_versions": {
                "secure-tunnel-core": dependency_version(sdk_cargo, "secure-tunnel-core"),
                "secure-tunnel-transport": dependency_version(
                    sdk_cargo,
                    "secure-tunnel-transport",
                ),
            },
            "sdk_ffi_dependency_versions": {
                "secure-tunnel-core": dependency_version(sdk_ffi_cargo, "secure-tunnel-core"),
                "secure-tunnel-sdk": dependency_version(sdk_ffi_cargo, "secure-tunnel-sdk"),
            },
        },
        "swift": {
            "package": regex_value(r'name:\s*"([^"]+)"', swift_text, "Swift package name"),
            "product": regex_value(r'\.library\(\s*name:\s*"([^"]+)"', swift_text, "Swift product"),
            "version_source": "git tag matching Rust workspace version",
        },
        "kotlin": {
            "group": regex_value(r'^\s*group\s*=\s*"([^"]+)"', kotlin_text, "Kotlin group"),
            "artifact": regex_value(r'^\s*artifactId\s*=\s*"([^"]+)"', kotlin_text, "Kotlin artifact"),
            "version": regex_value(r'^\s*version\s*=\s*"([^"]+)"', kotlin_text, "Kotlin version"),
        },
        "python": {
            "name": python_project.get("name"),
            "version": python_project.get("version"),
            "module": load_toml(root / "python/pyproject.toml")["tool"]["maturin"]["module-name"],
        },
        "flutter": {
            "name": flutter_pubspec.get("name"),
            "version": flutter_pubspec.get("version"),
            "publish_to": flutter_pubspec.get("publish_to"),
            "bridge_crate": flutter_rust_cargo["package"]["name"],
            "bridge_crate_version": flutter_rust_cargo["package"]["version"],
            "sdk_dependency_version": dependency_version(flutter_rust_cargo, "secure-tunnel-sdk"),
            "core_dependency_version": dependency_version(flutter_rust_cargo, "secure-tunnel-core"),
        },
        "go": {
            "module": regex_value(r"^module\s+(\S+)", go_mod, "Go module path"),
            "package": regex_value(r"^package\s+(\S+)", go_binding, "Go package name"),
            "version": regex_value(r'const\s+Version\s+=\s+"([^"]+)"', go_types, "Go version"),
        },
    }


def validate_metadata(metadata: dict[str, Any]) -> list[str]:
    version = metadata["version"]
    errors: list[str] = []

    for section, versions in (
        ("rust.sdk_dependency_versions", metadata["rust"]["sdk_dependency_versions"]),
        ("rust.sdk_ffi_dependency_versions", metadata["rust"]["sdk_ffi_dependency_versions"]),
    ):
        for name, dependency in versions.items():
            if dependency != version:
                errors.append(f"{section}.{name} = {dependency!r}, want {version!r}")

    expected = {
        "swift.package": (metadata["swift"]["package"], "SecureTunnel"),
        "swift.product": (metadata["swift"]["product"], "SecureTunnel"),
        "kotlin.group": (metadata["kotlin"]["group"], "io.github.asimihsan"),
        "kotlin.artifact": (metadata["kotlin"]["artifact"], "secure-tunnel-kotlin"),
        "kotlin.version": (metadata["kotlin"]["version"], version),
        "python.name": (metadata["python"]["name"], "secure-tunnel"),
        "python.version": (metadata["python"]["version"], version),
        "python.module": (metadata["python"]["module"], "secure_tunnel._native"),
        "flutter.name": (metadata["flutter"]["name"], "secure_tunnel_flutter"),
        "flutter.version": (metadata["flutter"]["version"], version),
        "flutter.publish_to": (metadata["flutter"]["publish_to"], "none"),
        "flutter.bridge_crate_version": (metadata["flutter"]["bridge_crate_version"], version),
        "flutter.sdk_dependency_version": (metadata["flutter"]["sdk_dependency_version"], version),
        "flutter.core_dependency_version": (metadata["flutter"]["core_dependency_version"], version),
        "go.module": (metadata["go"]["module"], GO_MODULE_PATH),
        "go.package": (metadata["go"]["package"], "securetunnel"),
        "go.version": (metadata["go"]["version"], version),
    }
    for label, (actual, wanted) in expected.items():
        if actual != wanted:
            errors.append(f"{label} = {actual!r}, want {wanted!r}")
    return errors


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def artifact(path: Path, package: str, root: Path) -> Artifact:
    return Artifact(
        package=package,
        path=path.relative_to(root).as_posix(),
        sha256=sha256_file(path),
        size_bytes=path.stat().st_size,
    )


def should_exclude(path: Path, excluded_names: set[str]) -> bool:
    return any(part in excluded_names for part in path.parts)


def iter_tar_files(source: Path, excluded_names: set[str]) -> list[Path]:
    if source.is_file():
        return [source]
    files: list[Path] = []
    for path in sorted(source.rglob("*")):
        if should_exclude(path.relative_to(source), excluded_names):
            continue
        if path.is_file():
            files.append(path)
    return files


def add_file_to_tar(tar: tarfile.TarFile, source: Path, arcname: PurePosixPath) -> None:
    info = tarfile.TarInfo(arcname.as_posix())
    info.size = source.stat().st_size
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    mode = source.stat().st_mode
    info.mode = 0o755 if mode & stat.S_IXUSR else 0o644
    tar.addfile(info, fileobj=source.open("rb"))


def create_tar(
    archive: Path,
    entries: list[tuple[Path, PurePosixPath]],
    excluded_names: set[str] | None = None,
) -> None:
    excluded = excluded_names or set()
    archive.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "w") as tar:
        for source, prefix in entries:
            for file_path in iter_tar_files(source, excluded):
                if source.is_file():
                    add_file_to_tar(tar, file_path, prefix)
                else:
                    relative = PurePosixPath(file_path.relative_to(source).as_posix())
                    add_file_to_tar(tar, file_path, prefix / relative)


def host_go_platform() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()
    os_name = {
        "darwin": "darwin",
        "linux": "linux",
        "windows": "windows",
    }.get(system)
    arch = {
        "aarch64": "arm64",
        "arm64": "arm64",
        "amd64": "amd64",
        "x86_64": "amd64",
    }.get(machine)
    if os_name is None or arch is None:
        raise ValueError(f"unsupported Go native release host: {system}-{machine}")
    return f"{os_name}-{arch}"


def go_native_library_name(go_platform: str) -> str:
    if go_platform.startswith("windows-"):
        return "secure_tunnel_ffi.dll"
    if go_platform.startswith("darwin-"):
        return "libsecure_tunnel_ffi.dylib"
    return "libsecure_tunnel_ffi.so"


def stage_go_module(root: Path, version: str) -> tuple[Path, dict[str, Any]]:
    go_platform = host_go_platform()
    library_name = go_native_library_name(go_platform)
    native_source = root / "target/release" / library_name
    if not native_source.exists():
        raise FileNotFoundError(f"missing Go native library: {native_source}")

    module_root = root / "target/sdk-release/staging/secure_tunnel_go"
    if module_root.exists():
        shutil.rmtree(module_root)
    module_root.mkdir(parents=True)
    for path in sorted((root / "crates/go").glob("*.go")):
        shutil.copy2(path, module_root / path.name)
    for relative in ("go.mod", "binding.h", "README.md"):
        shutil.copy2(root / "crates/go" / relative, module_root / relative)
    shutil.copy2(root / "LICENSE", module_root / "LICENSE")

    native_dir = module_root / "native" / go_platform
    native_dir.mkdir(parents=True)
    native_destination = native_dir / library_name
    shutil.copy2(native_source, native_destination)

    native_sha = sha256_file(native_destination)
    metadata = {
        "schema": "secure-tunnel-go-native-v1",
        "module": GO_MODULE_PATH,
        "package": "securetunnel",
        "version": version,
        "abi_version": "1",
        "platforms": [
            {
                "goos_goarch": go_platform,
                "library": f"native/{go_platform}/{library_name}",
                "sha256": native_sha,
                "size_bytes": native_destination.stat().st_size,
            },
        ],
    }
    (module_root / "native.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return module_root, metadata


def selected_packages(raw: str | None) -> list[str]:
    value = raw or os.environ.get("SECURE_TUNNEL_SDK_RELEASE_PACKAGES") or ",".join(PACKAGE_NAMES)
    packages = [item.strip() for item in value.split(",") if item.strip()]
    unknown = sorted(set(packages) - set(PACKAGE_NAMES))
    if unknown:
        raise ValueError(f"unknown SDK release package(s): {', '.join(unknown)}")
    return packages


def create_package_artifacts(root: Path, packages: list[str], version: str) -> list[Artifact]:
    output = root / "target/sdk-release"
    artifact_dir = output / "artifacts"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    artifacts: list[Artifact] = []

    if "python" in packages:
        wheels = sorted((root / "python/dist").glob("secure_tunnel-*.whl"))
        if not wheels:
            raise FileNotFoundError("missing Python wheel under python/dist")
        for path in wheels:
            destination = artifact_dir / path.name
            shutil.copy2(path, destination)
            artifacts.append(artifact(destination, "python", root))

    tar_specs: dict[str, tuple[Path, list[tuple[Path, PurePosixPath]], set[str]]] = {
        "swift": (
            artifact_dir / f"SecureTunnel-{version}.tar",
            [(root / "target/sdk/swift/SecureTunnel", PurePosixPath("SecureTunnel"))],
            {".build", ".swiftpm", "DerivedData"},
        ),
        "kotlin": (
            artifact_dir / f"secure-tunnel-kotlin-maven-{version}.tar",
            [(root / "target/sdk/kotlin/maven", PurePosixPath("maven"))],
            set(),
        ),
        "flutter": (
            artifact_dir / f"secure_tunnel_flutter-{version}.tar",
            [(root / "target/sdk/flutter/secure_tunnel_flutter", PurePosixPath("secure_tunnel_flutter"))],
            {
                ".dart_tool",
                ".flutter-plugins-dependencies",
                ".packages",
                ".pub-cache",
                "build",
                "pubspec.lock",
                "target",
            },
        ),
    }
    if "go" in packages:
        go_module, go_native = stage_go_module(root, version)
        (artifact_dir / f"secure_tunnel_go-{version}.native.json").write_text(
            json.dumps(go_native, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        artifacts.append(artifact(artifact_dir / f"secure_tunnel_go-{version}.native.json", "go", root))
        tar_specs["go"] = (
            artifact_dir / f"secure_tunnel_go-{version}.tar",
            [(go_module, PurePosixPath("secure_tunnel_go"))],
            set(),
        )
    for package in packages:
        if package == "python":
            continue
        archive, entries, excluded = tar_specs[package]
        missing = [source for source, _prefix in entries if not source.exists()]
        if missing:
            raise FileNotFoundError(
                f"missing {package} release input(s): "
                + ", ".join(path.as_posix() for path in missing),
            )
        create_tar(archive, entries, excluded)
        artifacts.append(artifact(archive, package, root))
    return artifacts


def command_check_metadata(root: Path) -> int:
    metadata = package_metadata(root)
    errors = validate_metadata(metadata)
    if errors:
        for error in errors:
            print(error)
        return 1
    print(json.dumps(metadata, indent=2, sort_keys=True))
    return 0


def command_manifest(root: Path, package_csv: str | None) -> int:
    metadata = package_metadata(root)
    errors = validate_metadata(metadata)
    if errors:
        for error in errors:
            print(error)
        return 1
    packages = selected_packages(package_csv)
    artifacts = create_package_artifacts(root, packages, metadata["version"])
    manifest = {
        "schema": "secure-tunnel-sdk-release-manifest-v1",
        "selected_packages": packages,
        "metadata": metadata,
        "artifacts": [item.__dict__ for item in sorted(artifacts, key=lambda value: value.path)],
    }
    output = root / "target/sdk-release"
    output.mkdir(parents=True, exist_ok=True)
    (output / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    checksums = "".join(f"{item.sha256}  {item.path}\n" for item in artifacts)
    (output / "checksums.txt").write_text(checksums, encoding="utf-8")
    print(f"Wrote SDK release manifest for {', '.join(packages)} to {output}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", type=Path)
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("check-metadata")
    manifest = subparsers.add_parser("manifest")
    manifest.add_argument("--packages", default=None)
    args = parser.parse_args()

    root = args.root.resolve()
    if args.command == "check-metadata":
        return command_check_metadata(root)
    if args.command == "manifest":
        return command_manifest(root, args.packages)
    raise AssertionError(f"unhandled command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
