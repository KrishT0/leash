# The Leash website

The public site lives here. It is plain HTML, CSS and JavaScript, with no build
step, so Cloudflare serves the files as they are.

Cloudflare Pages watches this repository. A push to `main` redeploys the site,
and a push to any other branch gives a preview URL for that branch. None of that
needs a workflow file in this repository, and none of it needs an API token,
because the connection is made once in the Cloudflare dashboard.

## Settings to enter in the dashboard

1. Create a Pages project and connect the GitHub repository.
2. Production branch: `main`.
3. Build command: leave this empty. There is nothing to build.
4. Build output directory: `/`. If the dashboard refuses that, leave it empty,
   which means the same thing here, because the build root is already `website`.
5. Root directory, meaning the folder Cloudflare builds from: `website`.

## What is in this folder

`index.html` is the page. The markup, the styles and the script all live in that
one file. It asks one other host for two things, the GSAP scripts from cdnjs that
drive the motion. The typefaces come from this folder, so nothing is loaded from
Google and no preconnect to it is needed.

`fonts` holds three families, latin subset only, which is all the page needs:

- Rubik, one variable file covering 400 to 900.
- Instrument Serif italic.
- JetBrains Mono, one variable file covering 100 to 800, copied from the app, which
  already serves the same file.

All three are under the SIL Open Font Licence, and the licence text sits beside
each one, as OFL-InstrumentSerif.txt, OFL-JetBrainsMono.txt and OFL-Rubik.txt.

`icon.svg` is the app mark, and it is what a browser tab shows.

`_headers` is read by Cloudflare Pages and applied to every response. It turns
off content type sniffing, keeps the referrer inside the origin, refuses to be
framed, and denies the location, camera and microphone permissions.

There is no content security policy on purpose. The page carries inline styles
and an inline script, so a policy for it needs `unsafe-inline` to work at all,
which is most of what such a policy is for. Moving the script into its own file is
the change that would make a real one possible. Self-hosted fonts already removed
one third-party host. cdnjs is the one that remains.

## The placeholders a release fills

`index.html` holds four placeholders in three values, and only a release can fill
them. Do not fill them from a local build: the installer carries its build time, so
two builds of the same commit have different bytes, different sizes and different
hashes. A hash on the page that does not match the file people downloaded is worse
than no hash at all.

| Placeholder | Where it appears | Where the value comes from |
| --- | --- | --- |
| `[VERSION]` | twice, in the hero and in the download row | the tag without its `v`. The release workflow checks it against all four version fields |
| `[FILE SIZE]` | the download row | the released file |
| `[SHA-256]` | the download row | the released file |

The download button is already filled in. It points at
`https://github.com/KrishT0/leash/releases/latest/download/Leash-setup.exe`, and
the release workflow gives the installer exactly that name, so the link never
changes between releases.

The release workflow writes the three values above into the run summary, so after
a tag has built they are in one place, ready to paste.

Until then a deploy shows the placeholders as they are. That is deliberate, and
the first release is expected before the page is worth reading.
