# Winget and Scoop

Version 0.3 · Size S · Touches ci, docs

## Why

`winget install` is how a new Windows machine gets set up. Scoop is the
developer's equivalent. Both are a manifest and an automation step.

## Experience

```powershell
winget install AkshayBajpai.KeepMeAwake
scoop bucket add bajpai https://github.com/ax5hay/scoop-bucket
scoop install keepmeawake
```

Both install the NSIS build silently (`/S`) per user.

## Winget

- Identifier `AkshayBajpai.KeepMeAwake`, installer type `nullsoft`, scope
  `user`, silent switch `/S`.
- Automation: `vedantmgoyal9/winget-releaser` runs on `release: published`,
  reads the NSIS asset, writes the three manifest files and opens the PR to
  `microsoft/winget-pkgs` with a PAT stored as `WINGET_TOKEN`.
- First submission is manual (`wingetcreate new`), because the identifier must
  exist before the releaser can update it. Commit the generated manifests under
  `packaging/winget/` for reference.

## Scoop

- New repo `ax5hay/scoop-bucket` with `bucket/keepmeawake.json`:

  ```json
  {
    "version": "0.3.0",
    "description": "Keeps your Windows laptop awake, politely.",
    "homepage": "https://akshaybajpai.com",
    "license": "MIT",
    "url": "https://github.com/ax5hay/KeepMeAwake/releases/download/v0.3.0/KeepMeAwake_0.3.0_x64-setup.exe",
    "hash": "<sha256>",
    "innosetup": false,
    "installer": { "args": ["/S"] },
    "checkver": "github",
    "autoupdate": {
      "url": "https://github.com/ax5hay/KeepMeAwake/releases/download/v$version/KeepMeAwake_$version_x64-setup.exe"
    }
  }
  ```

- Scoop's `checkver`/`autoupdate` keep it current; the bucket repo gets the
  standard `excavator` workflow.

## CI

- `release.yml`: after the draft is published (separate workflow on
  `release: published`): compute the SHA256 of the NSIS asset, print it in the
  job summary, run the winget releaser, and dispatch the scoop bucket's
  excavator.

## Docs

README install section gets the two commands; releasing gets the channels
table updated; changelog.

## Tests

None in-repo; the acceptance list is the test.

## Acceptance

- [ ] `winget install AkshayBajpai.KeepMeAwake` on a clean machine installs
      and launches the app; `winget upgrade` picks up the next release.
- [ ] `scoop install keepmeawake` does the same; `scoop update` picks up the
      next release.

## Commits

1. `ci: open a winget manifest PR on each published release`
2. `docs: add winget and scoop install commands`
3. (bucket repo) `feat: add keepmeawake manifest`

## Out of scope

Chocolatey, the Microsoft Store.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
