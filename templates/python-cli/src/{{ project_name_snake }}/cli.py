"""CLI entry point for {{ project_name }}."""

import click

from . import __version__


@click.group()
@click.version_option(version=__version__)
def main():
    """{{ description }}"""


@main.command()
@click.option("--name", default="world", help="Name to greet.")
def hello(name: str):
    """Say hello."""
    click.echo(f"Hello, {name}!")


if __name__ == "__main__":
    main()
