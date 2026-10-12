# Writes website/og.png, the card a shared link shows, at exactly 1200 by 630.
#
# python scripts/og-card.py
#
# The card is rendered from the site's own typefaces and colours rather than drawn
# by hand, so it cannot drift from the page. It writes scripts/og-card.html with the
# fonts embedded, then Chrome turns that into the image.

import base64
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
FONTS = ROOT / "website" / "fonts"
OUT = ROOT / "website" / "og.png"
TEMPLATE = ROOT / "scripts" / "og-card.html"
CHROME = pathlib.Path(r"C:\Program Files\Google\Chrome\Application\chrome.exe")

TEMPLATE_CSS = """@font-face {
                font-family: Rubik;
                font-style: normal;
                font-weight: 400 900;
                src: url(__RUBIK__) format("woff2");
            }
            @font-face {
                font-family: "Instrument Serif";
                font-style: italic;
                font-weight: 400;
                src: url(__INSTRUMENT__) format("woff2");
            }
            @font-face {
                font-family: "JetBrains Mono";
                font-style: normal;
                font-weight: 100 800;
                src: url(__MONO__) format("woff2");
            }"""

HTML = """<!doctype html>
<html lang="en">
    <head>
        <meta charset="utf-8" />
        <title>Leash, the card a shared link shows</title>
        <style>
            __FONTS__
            * {
                box-sizing: border-box;
                margin: 0;
                padding: 0;
            }
            html,
            body {
                width: 1200px;
                height: 630px;
                overflow: hidden;
            }
            body {
                background: #101216;
                color: #f3f4f6;
                font-family: Rubik, sans-serif;
            }
            .card {
                position: relative;
                width: 1200px;
                height: 630px;
                padding: 0 64px;
            }
            /* The frame the site uses: a rule on each side, and ticks like the rail. */
            .rule {
                position: absolute;
                top: 0;
                bottom: 0;
                width: 1px;
                background: #2a2d34;
            }
            .rule.left {
                left: 64px;
            }
            .rule.right {
                right: 64px;
            }
            .ticks {
                position: absolute;
                right: 64px;
                top: 53px;
                bottom: 0;
                width: 14px;
                background: repeating-linear-gradient(
                        to bottom,
                        transparent 0 39px,
                        #2a2d34 39px 40px
                    )
                    right / 14px 100% no-repeat;
            }
            .bar {
                display: flex;
                align-items: center;
                justify-content: space-between;
                height: 53px;
                border-bottom: 1px solid #2a2d34;
            }
            .lbl {
                font-family: "JetBrains Mono", monospace;
                font-weight: 500;
                font-size: 12px;
                letter-spacing: 0.14em;
                text-transform: uppercase;
                color: #8b909a;
            }
            h1 {
                margin-top: 72px;
                font-size: 80px;
                font-weight: 800;
                line-height: 1.02;
                letter-spacing: -0.035em;
            }
            h1 span {
                display: block;
            }
            h1 em {
                font-family: "Instrument Serif", Georgia, serif;
                font-style: italic;
                font-weight: 400;
                font-size: 1.12em;
                letter-spacing: -0.01em;
            }
            .lead {
                margin-top: 30px;
                max-width: 30ch;
                color: #8b909a;
                font-size: 22px;
                line-height: 1.5;
            }
            .ladder {
                margin-top: 34px;
                display: flex;
                gap: 14px;
                align-items: center;
            }
            .ladder b {
                font-weight: 500;
                color: #8b909a;
            }
            .ladder i {
                font-style: normal;
                color: #2a2d34;
            }
            .foot {
                position: absolute;
                left: 64px;
                right: 64px;
                bottom: 42px;
                display: flex;
                align-items: center;
                justify-content: space-between;
                padding-top: 18px;
                border-top: 1px solid #2a2d34;
            }
            .wordmark {
                display: flex;
                align-items: center;
                gap: 12px;
                font-size: 20px;
                font-weight: 800;
                letter-spacing: -0.01em;
            }
        </style>
    </head>
    <body>
        <div class="card">
            <div class="rule left"></div>
            <div class="rule right"></div>
            <div class="ticks"></div>
            <div class="bar">
                <span class="lbl">Leash</span>
                <span class="lbl">For Windows 11</span>
            </div>
            <h1>
                <span>Keep Windows</span>
                <span><em>services</em> stopped.</span>
            </h1>
            <p class="lead">
                You pick the services. Leash watches them. When one starts, Leash
                stops it again.
            </p>
            <div class="ladder lbl">
                <b>Notify only</b><i>/</i><b>Dry run</b><i>/</i><b>Stop</b>
            </div>
            <div class="foot">
                <span class="wordmark">
                    <svg width="20" height="20" viewBox="-12 -12 24 24" aria-hidden="true">
                        <circle
                            r="10"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.6"
                        />
                        <circle cx="4" cy="-3" r="3" fill="currentColor" />
                    </svg>
                    Leash
                </span>
                <span class="lbl">leash.krisht0.workers.dev</span>
            </div>
        </div>
    </body>
</html>
"""


def data_uri(path):
    encoded = base64.b64encode(path.read_bytes()).decode("ascii")
    return f"data:font/woff2;base64,{encoded}"


def main():
    fonts = TEMPLATE_CSS
    fonts = fonts.replace("__RUBIK__", data_uri(FONTS / "rubik-latin.woff2"))
    fonts = fonts.replace(
        "__INSTRUMENT__", data_uri(FONTS / "instrument-serif-italic-latin.woff2")
    )
    fonts = fonts.replace(
        "__MONO__", data_uri(FONTS / "jetbrains-mono-latin.woff2")
    )

    html = HTML.replace("__FONTS__", fonts)
    TEMPLATE.write_text(html, encoding="utf-8")
    print(f"wrote {TEMPLATE.relative_to(ROOT)} ({len(html) // 1024} KB with the fonts inside)")

    if not CHROME.exists():
        print(f"Chrome is not at {CHROME}, so the render was skipped", file=sys.stderr)
        return 1

    subprocess.run(
        [
            str(CHROME),
            "--headless=new",
            "--disable-gpu",
            "--hide-scrollbars",
            "--force-device-scale-factor=1",
            "--virtual-time-budget=3000",
            "--window-size=1200,630",
            f"--screenshot={OUT}",
            TEMPLATE.as_uri(),
        ],
        check=True,
        capture_output=True,
        timeout=180,
    )
    print(f"rendered {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
