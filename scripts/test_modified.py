#!/usr/bin/env python3

from typing import List
import os
from os import path
import logging
import shlex
import subprocess
import sys
import time


LOG_FORMAT = "%(asctime)-15s - %(levelname)s - %(message)s"


def execute_command(command: str):
    start_time = time.time()
    logging.info("executing %s...", command)
    command_args = shlex.split(command)
    retcode = subprocess.call(command_args)
    ellapsed = time.time() - start_time
    logging.info("executed %s in %.2fs", command, ellapsed)
    if retcode != 0:
        logging.error("command failed with code %s", retcode)
    return retcode


class Test:
    def __init__(self, name: str, root_dir: str,
                 test_commands: List[str], extra_dirs: List[str] = None):
        self.name = name
        self.root_dir = path.abspath(root_dir)
        self.test_commands = test_commands
        self._executed = False
        if extra_dirs is None:
            extra_dirs = []
        self.extra_dirs = [path.abspath(directory) for directory in extra_dirs]

    def run(self):
        logging.info("running test %s", self.name)
        os.chdir(self.root_dir)
        for command in self.test_commands:
            retcode = execute_command(command)
            if retcode != 0:
                return retcode
        logging.info("test %s succeeded", self.name)
        self._executed = True
        return 0

    def should_run(self, modified_files: List[str]):
        if self._executed:
            return False
        directories_to_check = [self.root_dir] + self.extra_dirs
        for directory in directories_to_check:
            if any(filename.startswith(directory) for filename in modified_files):
                return True
        return False


TESTS = [
    Test("bitcoin", "crates/bitcoin", [
        "cargo build --release",
        "cargo test --verbose",
    ]),
    Test("btc-relay", "crates/btc-relay", [
        "cargo build --release",
        "cargo test --verbose",
    ], ["crates/security"]),
    Test("security", "crates/security", [
        "cargo build --release",
        "cargo test --verbose",
    ]),
]


def get_modified_files():
    command = ["git", "diff", "--name-only", "HEAD", "HEAD~1"]
    process = subprocess.Popen(command, stdout=subprocess.PIPE)
    stdout, _stderr = process.communicate()
    if process.returncode != 0:
        return process.returncode, []
    return 0, [path.abspath(s.decode("utf-8")) for s in stdout.splitlines()]


def main():
    logging.basicConfig(level=logging.INFO, format=LOG_FORMAT)

    return_code, files = get_modified_files()
    if return_code != 0:
        return return_code
    for test in TESTS:
        if test.should_run(files):
            retcode = test.run()
            if retcode != 0:
                return retcode
    return 0


if __name__ == "__main__":
    sys.exit(main())
