# fledge — marketing site

Astro + MDX site. Hosted on GitHub Pages at https://corvidlabs.github.io/fledge.

## Dev

    bun install
    bun run dev        # localhost:4321/fledge/

## Build

    bun run build      # writes site/dist/

## Test

    bun test

## Dev gotcha — `plugins.json` shows as modified after build

`site/src/data/plugins.json` is committed as an empty seed (`[]`) so the dev
server runs without a network fetch on a fresh clone. The prebuild script
rewrites it with real registry data. Because the path is also gitignored,
`git status` will keep showing it as modified after every build.

To silence the noise on your local checkout (this only affects you, not the
repo state):

    git update-index --skip-worktree site/src/data/plugins.json

To re-enable tracking (if you actually want to edit the seed):

    git update-index --no-skip-worktree site/src/data/plugins.json
