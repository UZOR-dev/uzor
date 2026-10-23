# UzorEmoji subset

`UzorEmoji-subset.woff2` — Noto Color Emoji (OFL 1.1, Google) cut down to the code points in
`UzorEmoji-subset.unicodes.txt` (reaction picker set, smileys, hands, hearts and common status
symbols; ~180 code points, ~440 KB). It is the web host's last-resort colour-emoji face
(`uzor_render_canvas2d::emoji`): served next to the page and fetched only when the platform
has no colour-emoji font of its own. Never compiled into a wasm.

Regenerate (fontTools + brotli):

    pyftsubset ../../uzor-fonts-emoji/fonts/NotoColorEmoji.ttf --unicodes-file=UzorEmoji-subset.unicodes.txt \
      --layout-features='*' --flavor=woff2 --output-file=UzorEmoji-subset.woff2
