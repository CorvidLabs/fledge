# Versioning — Context

## Problem

Templates can declare `min_fledge_version` but fledge doesn't enforce it. Users can get confusing errors when a template requires features from a newer version.

## Solution

A small versioning module with semver comparison. The init flow checks the constraint before scaffolding and gives a clear upgrade message if incompatible.
