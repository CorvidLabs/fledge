"""Tests for the CLI."""

from click.testing import CliRunner
from {{ project_name_snake }}.cli import main


def test_hello_default():
    runner = CliRunner()
    result = runner.invoke(main, ["hello"])
    assert result.exit_code == 0
    assert "Hello, world!" in result.output


def test_hello_with_name():
    runner = CliRunner()
    result = runner.invoke(main, ["hello", "--name", "fledge"])
    assert result.exit_code == 0
    assert "Hello, fledge!" in result.output


def test_version():
    runner = CliRunner()
    result = runner.invoke(main, ["--version"])
    assert result.exit_code == 0
    assert "0.1.0" in result.output
