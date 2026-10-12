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
3. Build command: leave this empty. There is nothing to build. If the dashboard
   refuses an empty field, use `exit 0`, which is the documented stand-in for a
   project with no build step.
4. Build output directory: `/`. If the dashboard refuses that, leave it empty,
   which means the same thing here, because the build root is already `website`.
   `wrangler.jsonc` sets this field, so the dashboard may not offer it. See the
   note on that file below.
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

`robots.txt` allows the whole site and points at the sitemap. `sitemap.xml` holds
one entry, because there is one page.

`wrangler.jsonc` names the project and sets `pages_build_output_dir`, the folder
the built site is taken from. Cloudflare documents this file as living in the
Pages project directory, which is this folder, and as being written there by
`wrangler pages download config`.

It is not required. A project connected by git integration takes its build
settings from the dashboard, and `wrangler pages deploy <directory>` takes the
directory on the command line. It is also not free of consequence: once the file
holds `pages_build_output_dir`, Cloudflare documents it as the source of truth for
the project, with the same fields no longer editable in the dashboard, and warns
that Pages will then use the configuration written for local use, which is often
not the production configuration. Ours holds the project name and the output
folder only, both of which match the dashboard, so the two cannot disagree today.
Delete the file if you would rather keep the dashboard in charge.

Two files are still owed, and the tags for them are already in the page:

- `404.html`. Cloudflare Pages serves the root page for any unknown path unless a
  `404.html` sits at the top level of the output. Without it, a wrong address
  answers 200 with the landing page, which is bad for search engines as well as
  for the visitor.
- `og.png`, a 1200 by 630 image for link previews. Until it exists, a shared link
  shows the title and description with no picture.

`_headers` is read by Cloudflare Pages and applied to every response. It turns
off content type sniffing, keeps the referrer inside the origin, refuses to be
framed, and denies the location, camera and microphone permissions.

There is no content security policy on purpose. The page carries inline styles
and an inline script, so a policy for it needs `unsafe-inline` to work at all,
which is most of what such a policy is for. Moving the script into its own file is
the change that would make a real one possible. Self-hosted fonts already removed
one third-party host. cdnjs is the one that remains.

## The domain

One value lives in four places, and a change has to reach all four:

- `https://leash.pages.dev/` in the canonical link, `og:url` and the structured
  data in `index.html`
- `https://leash.pages.dev/og.png` in the two image tags in `index.html`
- `https://leash.pages.dev/` in `sitemap.xml`
- `https://leash.pages.dev/sitemap.xml` in `robots.txt`

The value assumes the Pages project is called `leash`, which is what
`wrangler.jsonc` says. A custom domain replaces it in those places and nowhere
else.

## Search engines

`index.html` carries a canonical address, an Open Graph block with a 1200 by 630
image, a large Twitter card, and structured data of type `SoftwareApplication`
with the operating system, the licence and the download address. `robots.txt`
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
| `softwareVersion` | in the structured data, not a placeholder | the same version, with no dots removed and no `v` |
| `[FILE SIZE]` | the download row | the released file |
| `[SHA-256]` | the download row | the released file |

The download button is already filled in. It points at
`https://github.com/KrishT0/leash/releases/latest/download/Leash-setup.exe`, and
the release workflow gives the installer exactly that name, so the link never
changes between releases.

The release workflow writes the values above into the run summary, so after a tag
has built they are in one place, ready to paste.

Until then a deploy shows the placeholders as they are. That is deliberate, and
the first release is expected before the page is worth reading.
