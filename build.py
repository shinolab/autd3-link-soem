#!/usr/bin/env python3

import argparse
import os
import re
import sys
from pathlib import Path
from typing import Optional

from tools.autd3_build_utils.autd3_build_utils import (
    BaseConfig,
    err,
    fetch_submodule,
    info,
    rm_glob_f,
    run_command,
    with_env,
    working_dir,
)


class Config(BaseConfig):
    release: bool
    target: Optional[str]
    no_examples: bool
    channel: Optional[str]
    features: str

    def __init__(self, args) -> None:
        super().__init__()

        self.release = hasattr(args, "release") and args.release
        self.no_examples = hasattr(args, "no_examples") and args.no_examples
        self.channel = hasattr(args, "channel") and args.channel
        self.features = args.features if hasattr(args, "features") and args.features else ""

        if hasattr(args, "arch") and args.arch is not None:
            if self.is_linux():
                match args.arch:
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
                match args.arch:
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
            command.append("cargo")
            command.extend(subcommands)
        else:
            if self.is_linux():
                command.append("cross")
                command.extend(subcommands)
            else:
                command.append("cargo")
                command.extend(subcommands)
            command.append("--target")
            command.append(self.target)
        if self.release:
            command.append("--release")
        features = self.features + " remote"
        command.append("--features")
        command.append(features)
        return command

    def setup_linker(self):
        if not self.is_linux() or self.target is None:
            return

        os.makedirs(".cargo", exist_ok=True)
        with open(".cargo/config", "w") as f:
            if self.target == "armv7-unknown-linux-gnueabihf":
                f.write("[target.armv7-unknown-linux-gnueabihf]\n")
                f.write('linker = "arm-linux-gnueabihf-gcc"\n')
            if self.target == "aarch64-unknown-linux-gnu":
                f.write("[target.aarch64-unknown-linux-gnu]\n")
                f.write('linker = "aarch64-linux-gnu-gcc"\n')


def rust_build(args):
    config = Config(args)

    command = config.cargo_command(["build"])
    if not config.no_examples:
        command.append("--examples")
    run_command(command)


def rust_lint(args):
    config = Config(args)

    command = config.cargo_command(["clippy"])
    command.append("--tests")
    if not config.no_examples:
        command.append("--examples")
    command.append("--")
    command.append("-D")
    command.append("warnings")
    run_command(command)


def rust_test(args):
    config = Config(args)

    if args.miri:
        with with_env(MIRIFLAGS="-Zmiri-disable-isolation"):
            miri_channel = args.channel if args.channel is not None else "nightly"
            command = config.cargo_command([f"+{miri_channel}", "miri", "nextest", "run"])
            run_command(command)
    else:
        command = config.cargo_command(["nextest", "run"])
        run_command(command)


def rust_run(args):
    examples = [
        "soem",
        "remote_soem",
    ]

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
        commands.append("--example")
        commands.append(args.target)
        commands.append("--no-default-features")
        if features is not None:
            commands.append("--features")
            commands.append(features)
        run_command(commands)


def rust_clear(_):
    run_command(["cargo", "clean"])


def rust_coverage(args):
    config = Config(args)

    with with_env(
        RUSTFLAGS="-C instrument-coverage",
        LLVM_PROFILE_FILE="%m-%p.profraw",
    ):
        command = config.cargo_command(["build"])

        run_command(command)
        command[1] = "test"
        run_command(command)

        command = [
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
            r"GRCOV_EXCL_LINE|^\s*\.await;?$|#\[derive|#\[error|#\[bitfield_struct|unreachable!|unimplemented!|tracing::(debug|trace|info|warn|error)!\([\s\S]*\);",
            "--keep-only",
            "src/**/*.rs",
            "--excl-start",
            "GRCOV_EXCL_START",
            "--excl-stop",
            "GRCOV_EXCL_STOP",
        ]
        run_command(command)
        rm_glob_f("**/*.profraw")


def util_update_ver(args):
    version = args.version

    with open("Cargo.toml", "r") as f:
        content = f.read()
        content = re.sub(
            r'^version = "(.*?)"',
            f'version = "{version}"',
            content,
            flags=re.MULTILINE,
        )
        content = re.sub(
            r'^autd3(.*)version = "(.*?)"',
            f'autd3\\1version = "{version}"',
            content,
            flags=re.MULTILINE,
        )
    with open("Cargo.toml", "w") as f:
        f.write(content)


def util_glob_unsafe(_):
    path = Path.cwd()
    files = set(path.rglob("**/*.rs"))
    unsafe_files: list[str] = []
    for file_path in sorted(files):
        with open(file_path) as file:
            for line in file.readlines():
                if "unsafe" in line and "ignore miri" not in line:
                    unsafe_files.append(str(file_path.absolute()))
                    break
    with open("filelist-for-miri-test.txt", "w") as f:
        f.write("\n".join(str(file) for file in unsafe_files))


def command_help(args):
    print(parser.parse_args([args.command, "--help"]))


if __name__ == "__main__":
    fetch_submodule()

    with working_dir(os.path.dirname(os.path.abspath(__file__))):
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
