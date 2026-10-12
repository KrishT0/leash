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

`_headers` is read by Cloudflare Pages and applied to every response. It turns
off content type sniffing, keeps the referrer inside the origin, refuses to be
framed, and denies the location, camera and microphone permissions.

There is no content security policy on purpose. A policy written before the site
exists would be a guess, and a wrong one breaks the page it is meant to protect.
Add one once there is markup to write it against.

## Until the site exists

This folder has no `index.html` yet, so a deploy of it serves a 404. That is
expected, not a fault.
