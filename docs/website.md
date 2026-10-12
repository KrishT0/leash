# The website

The public site is the `website` folder. It is plain HTML, CSS and JavaScript with
no build step, so Cloudflare serves the files as they are.

It runs as a Cloudflare Worker with static assets and no Worker code.
`wrangler.jsonc` at the root of this repository is the whole configuration: the
name `leash`, the assets directory `./website`, and `not_found_handling` set to
`404-page`, which is what makes an unknown address return the 404 page instead of
the front page. The name and the account's workers.dev subdomain give the address,
https://leash.krisht0.workers.dev.

That file sits at the root rather than beside the site on purpose. Cloudflare's
documentation is explicit that the Worker name in the dashboard must match the
`name` in the configuration file in the build's root directory, or the build fails,
so the root is the directory to build from and the one to name in the dashboard.

## Deploying

Two ways, and they do not exclude each other.

Cloudflare's Git integration, called Workers Builds: in the dashboard, go to
Workers & Pages, then Create application, then Import a repository. The root
directory is the repository root, because that is where `wrangler.jsonc` lives. The
Worker name must be `leash`, matching the file. There is nothing to build, so the
build command can be left empty or set to `exit 0`, and the deploy command is
`npx wrangler deploy`, which reads the configuration file.

Or by hand, from the repository root:

```
npx wrangler deploy
```

A Worker that has static assets and no `main` in its configuration serves the files
and answers 404 for anything else. Adding Worker code later changes that, and any
custom headers would then have to be attached by the Worker rather than by
`_headers`, which applies to static assets only.

## The files

`index.html` is the landing page and `site.css` holds its styles. The typefaces
come from `fonts`, so nothing on this site is loaded from Google. The page asks one
other host for two things: the GSAP scripts from cdnjs that drive the motion.

`404.html` is shown for any address that does not exist. Cloudflare serves the
front page for an unknown address unless a top-level `404.html` is present, or
`not_found_handling` says otherwise, so without this file a wrong address would
answer 200 with the landing page. It is self-contained: its own smaller set of
styles, no shared stylesheet, and the same three typefaces declared from `fonts`,
so no page here loads a font from Google. It carries `noindex`, and it has one
small script that restores the chosen theme.

`fonts` holds three families, latin subset only, which is all the page needs, each
with its licence text beside it:

- Rubik, one variable file covering 400 to 900.
- Instrument Serif italic.
- JetBrains Mono, one variable file covering 100 to 800, copied from the app, which
  already serves the same file.

`icon.svg` is the app mark, and it is what a browser tab shows.

`og.png` is the 1200 by 630 card a shared link shows. `scripts/og-card.py` writes
it, rendering the site's own fonts and colours at exactly that size. Run it from
the repository root after a change to the headline, the palette or the address, and
commit the new image.

`robots.txt` allows the whole site and points at the sitemap. `sitemap.xml` holds
one entry, because there is one page.

`_headers` is read by Cloudflare and applied to static asset responses. It turns
off content type sniffing, keeps the referrer inside the origin, refuses to be
framed, and denies the location, camera and microphone permissions. Workers parse
this file and never serve it. If the site ever moves behind a custom domain, add
the line from Cloudflare's documentation that sends `X-Robots-Tag: noindex` on the
`workers.dev` host, so the two addresses are not indexed as duplicates.

There is no README in that folder on purpose. Everything in it is uploaded and
served, so only public files belong there. This document lives in `docs`.

There is no content security policy yet, and the styles are no longer the reason.
They moved out of the page into `site.css`, and `404.html` has no script at all.
What blocks a strict policy today is the landing page's inline script, which runs
the theme toggle, the rail and the motion, and the inline `application/ld+json`
block. A policy for those needs `unsafe-inline` or hashes. Self-hosted fonts
already removed one third-party host, and cdnjs is the one that remains.

## The domain

One value lives in three files, and a change has to reach all of them:

- `https://leash.krisht0.workers.dev/` in the canonical link, `og:url` and the
  structured data in `website/index.html`, and in `website/sitemap.xml`
- `https://leash.krisht0.workers.dev/og.png` in the two image tags in
  `website/index.html`
- `https://leash.krisht0.workers.dev/sitemap.xml` in `website/robots.txt`
- `leash.krisht0.workers.dev`, drawn on the card, in `scripts/og-card.py`

The address comes from the Worker name `leash` and the account's workers.dev
subdomain `krisht0`. A custom domain would replace it in those places and nowhere
else.

## Search engines

`index.html` carries a canonical address, an Open Graph block with a 1200 by 630
image, a large Twitter card, and structured data of type `SoftwareApplication`
holding the operating system, the licence and the download address. `robots.txt`
points at `sitemap.xml`, and the sitemap has one entry.

The structured data holds `softwareVersion`. That makes seven places the version
appears: two in the page, four in the manifests, and this one. It is in the table
below, with no `v` and no other change.

## The placeholders a release fills

`index.html` holds four placeholders in three values, and only a release can fill
them. Do not fill them from a local build: the installer carries its build time, so
two builds of the same commit have different bytes, different sizes and different
hashes. A hash on the page that does not match the file people downloaded is worse
than no hash at all.

| Placeholder | Where it appears | Where the value comes from |
| --- | --- | --- |
| `[VERSION]` | twice, in the hero and in the download row | the tag without its `v`. The release workflow checks it against all four version fields |
| `softwareVersion` | in the structured data, not a placeholder | the same version, with no `v` |
| `[FILE SIZE]` | the download row | the released file |
| `[SHA-256]` | the download row | the released file |

The download button is already filled in. It points at
`https://github.com/KrishT0/leash/releases/latest/download/Leash-setup.exe`, and
the release workflow gives the installer exactly that name, so the link never
changes between releases.

The release workflow writes the values above into the run summary, so after a tag
has built they are in one place, ready to paste.

Until then a deploy shows the placeholders as they are. That is deliberate, and the
first release is expected before the page is worth reading.
