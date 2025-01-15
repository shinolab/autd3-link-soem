#!/usr/bin/env python3

import argparse
import re
import sys
from pathlib import Path

from tools.autd3_build_utils.autd3_build_utils import (
    BaseConfig,
    err,
    fetch_submodule,
    info,
    rremove,
    run_command,
    substitute_in_file,
    with_env,
    working_dir,
)


class Config(BaseConfig):
    target: str | None
    no_examples: bool
    channel: str | None
    features: str

    def __init__(self, args) -> None:  # noqa: ANN001
        super().__init__(args)

        self.no_examples = getattr(args, "no_examples", False)
        self.channel = getattr(args, "channel", "nightly") or "nightly"
        self.features = getattr(args, "features", "") or ""

        arch: str = getattr(args, "arch", None)
        if arch:
            if self.is_linux():
                match arch:
                    case "":
                        self.target = None
                    case "arm32":
                        self.target = "armv7-unknown-linux-gnueabihf"
                    case "aarch64":
                        self.target = "aarch64-unknown-linux-gnu"
                    case _:
                        err(f'arch "{args.arch}" is not supported.')
                        sys.exit(-1)
            elif self.is_windows():
                match arch:
                    case "":
                        self.target = None
                    case "aarch64":
                        self.target = "aarch64-pc-windows-msvc"
                    case _:
                        err(f'arch "{args.arch}" is not supported.')
                        sys.exit(-1)
            else:
                self.target = None
        else:
            self.target = None

    def cargo_command(self, subcommands: list[str]) -> list[str]:
        command = []
        if self.target is None:
            command.extend(["cargo", *subcommands])
        else:
            if self.is_linux():
                command.extend(["cross", *subcommands])
            else:
                command.extend(["cargo", *subcommands])
            command.extend(["--target", self.target])
        if self.release:
            command.append("--release")
        command.extend(["--features", self.features + " remote blocking"])
        return command


def rust_build(args) -> None:  # noqa: ANN001
    config = Config(args)
    command = config.cargo_command(["build"])
    if not config.no_examples:
        command.append("--examples")
    run_command(command)


def rust_lint(args) -> None:  # noqa: ANN001
    config = Config(args)
    command = config.cargo_command(["clippy", "--tests"])
    if not config.no_examples:
        command.append("--examples")
    command.extend(["--", "-D", "warnings", "-W", "clippy::all"])
    run_command(command)


def rust_doc(_) -> None:  # noqa: ANN001
    with with_env(RUSTDOCFLAGS="--cfg docsrs -D warnings"):
        run_command(["cargo", "+nightly", "doc", "--no-deps"])


def rust_test(args) -> None:  # noqa: ANN001
    config = Config(args)
    if args.miri:
        with with_env(MIRIFLAGS="-Zmiri-disable-isolation"):
            run_command(config.cargo_command([f"+{config.channel}", "miri", "nextest", "run"]))
    else:
        run_command(config.cargo_command(["nextest", "run"]))


def rust_run(args):  # noqa: ANN001, ANN201
    examples = ["soem", "remote_soem"]
    if args.target not in examples:
        err(f'example "{args.target}" is not found.')
        info(f"Available examples: {examples}")
        return sys.exit(-1)
    features: str
    match args.target:
        case "soem":
            features = "local"
        case "remote_soem":
            features = "remote"
    if args.features is not None:
        features += " " + args.features
    with working_dir("examples"):
        commands = ["cargo", "run"]
        if args.release:
            commands.append("--release")
        commands.extend(["--example", args.target, "--no-default-features", "--features", features])
        if features is not None:
            commands.extend(["--features", features])
        run_command(commands)
        return None


def rust_clear(_) -> None:  # noqa: ANN001
    run_command(["cargo", "clean"])


def rust_coverage(args) -> None:  # noqa: ANN001
    config = Config(args)
    with with_env(
        RUSTFLAGS="-C instrument-coverage",
        LLVM_PROFILE_FILE="%m-%p.profraw",
    ):
        run_command(config.cargo_command(["build"]))
        run_command(config.cargo_command(["test"]))
        exclude_patterns = [
            "GRCOV_EXCL_LINE",
            r"#\[derive",
            r"#\[error",
            r"#\[bitfield_struct",
            r"unreachable!",
            r"unimplemented!",
            r"tracing::(debug|trace|info|warn|error)!\([\s\S]*\);",
        ]
        run_command(
            [
                "grcov",
                ".",
                "-s",
                ".",
                "--binary-path",
                "./target/debug",
                "--llvm",
                "--branch",
                "--ignore-not-existing",
                "-o",
                "./coverage",
                "-t",
                args.format,
                "--excl-line",
                "|".join(exclude_patterns),
                "--keep-only",
                "src/**/*.rs",
                "--excl-start",
                "GRCOV_EXCL_START",
                "--excl-stop",
                "GRCOV_EXCL_STOP",
            ]
        )
        rremove("**/*.profraw")


def util_update_ver(args) -> None:  # noqa: ANN001
    version = args.version
    substitute_in_file(
        "Cargo.toml",
        [(r'^version = "(.*?)"', f'version = "{version}"'), (r'^autd3(.*)version = "(.*?)"', f'autd3\\1version = "{version}"')],
        flags=re.MULTILINE,
    )


def util_glob_unsafe(_) -> None:  # noqa: ANN001
    path = Path.cwd()
    files = set(path.rglob("**/*.rs"))
    unsafe_files: list[str] = []
    for file_path in sorted(files):
        with file_path.open() as file:
            for line in file.readlines():
                if "unsafe" in line and "ignore miri" not in line:
                    unsafe_files.append(str(file_path.absolute()))
                    break
    with Path("filelist-for-miri-test.txt").open("w") as f:
        f.write("\n".join(str(file) for file in unsafe_files))


def command_help(args) -> None:  # noqa: ANN001
    print(parser.parse_args([args.command, "--help"]))


if __name__ == "__main__":
    with working_dir(Path(__file__).parent):
        fetch_submodule()

        parser = argparse.ArgumentParser(description="autd3 library build script")
        subparsers = parser.add_subparsers()

        # build
        parser_build = subparsers.add_parser("build", help="see `build -h`")
        parser_build.add_argument("--release", action="store_true", help="release build")
        parser_build.add_argument("--arch", help="cross-compile for specific architecture (for Linux)")
        parser_build.add_argument("--features", help="additional features", default=None)
        parser_build.add_argument("--no-examples", action="store_true", help="skip examples")
        parser_build.set_defaults(handler=rust_build)

        # lint
        parser_lint = subparsers.add_parser("lint", help="see `lint -h`")
        parser_lint.add_argument("--release", action="store_true", help="release build")
        parser_lint.add_argument("--features", help="additional features", default=None)
        parser_lint.add_argument("--no-examples", action="store_true", help="skip examples")
        parser_lint.set_defaults(handler=rust_lint)

        # doc
        parser_doc = subparsers.add_parser("doc", help="see `doc -h`")
        parser_doc.set_defaults(handler=rust_doc)

        # test
        parser_test = subparsers.add_parser("test", help="see `test -h`")
        parser_test.add_argument("--release", action="store_true", help="release build")
        parser_test.add_argument("--features", help="additional features", default=None)
        parser_test.add_argument("--miri", action="store_true", help="run with miri")
        parser_test.add_argument("--channel", help="rust toolchain", default=None)
        parser_test.set_defaults(handler=rust_test)

        # run
        parser_run = subparsers.add_parser("run", help="see `run -h`")
        parser_run.add_argument("target", help="binary target")
        parser_run.add_argument("--release", action="store_true", help="release build")
        parser_run.add_argument("--features", help="additional features", default=None)
        parser_run.set_defaults(handler=rust_run)

        # clear
        parser_clear = subparsers.add_parser("clear", help="see `clear -h`")
        parser_clear.set_defaults(handler=rust_clear)

        # coverage
        parser_cov = subparsers.add_parser("cov", help="see `cov -h`")
        parser_cov.add_argument("--format", help="output format (lcov|html|markdown)", default="lcov")
        parser_cov.set_defaults(handler=rust_coverage)

        # util
        parser_util = subparsers.add_parser("util", help="see `util -h`")
        subparsers_util = parser_util.add_subparsers()

        # util update version
        parser_util_upver = subparsers_util.add_parser("upver", help="see `util upver -h`")
        parser_util_upver.add_argument("version", help="version")
        parser_util_upver.set_defaults(handler=util_update_ver)

        # enumerate file which contains unsafe codes
        parser_glob_unsafe = subparsers_util.add_parser("glob_unsafe", help="see `util glob_unsafe -h`")
        parser_glob_unsafe.set_defaults(handler=util_glob_unsafe)

        # help
        parser_help = subparsers.add_parser("help", help="see `help -h`")
        parser_help.add_argument("command", help="command name which help is shown")
        parser_help.set_defaults(handler=command_help)

        args = parser.parse_args()
        if hasattr(args, "handler"):
            args.handler(args)
        else:
            parser.print_help()
