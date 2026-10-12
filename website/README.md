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

`index.html` is the page. It is one file: the markup, the styles and the script
all live in it, and it asks the browser for three things from elsewhere, the
Google Fonts stylesheet and two scripts from cdnjs for the motion.

`_headers` is read by Cloudflare Pages and applied to every response. It turns
off content type sniffing, keeps the referrer inside the origin, refuses to be
framed, and denies the location, camera and microphone permissions.

There is no content security policy on purpose. The page carries inline styles
and an inline script, so a policy for it needs `unsafe-inline` to work at all,
which is most of what the policy is for. Moving the script into its own file is
the change that would make a real policy possible, and that is a decision for
later, not a guess now.

## The five placeholders

`index.html` holds five placeholders that only a release can fill. Do not fill
them from a local build: the installer carries its build time, so two builds of
the same commit have different bytes, different sizes and different hashes. A hash
on the page that does not match the file people downloaded is worse than no hash
at all.

| Placeholder | Where it appears | Where the value comes from |
| --- | --- | --- |
| `[VERSION]` | twice, in the hero and in the download row | the tag without its `v`. It matches the four version fields, which the release workflow checks |
| `[DOWNLOAD URL]` | the download button | the release asset URL |
| `[FILE SIZE]` | the download row | the released file |
| `[SHA-256]` | the download row | the released file |

The release workflow writes all of these into the run summary, so after a tag has
built they are in one place, ready to paste.

Until the page is filled in, a deploy shows those placeholders as they are.
Would you rather the page never show them, say so and the download row can hide
itself until the values exist.
