# Publish — Requirements

## Functional Requirements

1. Validate that the target directory contains a parseable template.toml
2. Require a configured GitHub token
3. Create a new GitHub repository (personal or org) via the REST API
4. Set the `fledge-template` topic on the repository
5. Set repository description from template.toml
6. Push all template files to the repository
7. Display the `fledge init` install command on success
8. Support `--org` flag for publishing under an organization
9. Support `--private` flag for private repositories
10. Detect and handle existing repositories (prompt to update)

## Non-Functional Requirements

1. Clear error messages guiding users to fix issues (missing token, invalid template)
2. No interactive prompts except for the update confirmation (scriptable by default)
