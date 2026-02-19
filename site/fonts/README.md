Place font files in this directory for self-hosting:

- `Railway.woff2`
- `RailwayAlternate.woff2`
- `FiraCode-Regular.woff2`
- `FiraCode-Medium.woff2`

Suggested sources:
- Railway: https://github.com/walkerbrown/railway-sans
- Fira Code: https://github.com/walkerbrown/FiraCode

Build from source (reproducible local workflow):

```bash
git clone --depth 1 https://github.com/walkerbrown/railway-sans /tmp/railway-sans
git clone --depth 1 https://github.com/walkerbrown/FiraCode /tmp/FiraCode

python3 -m venv /tmp/fontbuild-venv
source /tmp/fontbuild-venv/bin/activate
python -m pip install --upgrade pip
python -m pip install fontmake fonttools brotli

cd /tmp/FiraCode
mkdir -p "distr/ttf/Fira Code"
fontmake -g FiraCode.glyphs -o ttf --output-path "distr/ttf/Fira Code/FiraCode-Regular.ttf" -i ".* Regular"
fontmake -g FiraCode.glyphs -o ttf --output-path "distr/ttf/Fira Code/FiraCode-Medium.ttf" -i ".* Medium"

python3 - <<'PY'
from pathlib import Path
from fontTools.ttLib import TTFont

mapping = {
    Path("/tmp/FiraCode/distr/ttf/Fira Code/FiraCode-Regular.ttf"): Path("site/fonts/FiraCode-Regular.woff2"),
    Path("/tmp/FiraCode/distr/ttf/Fira Code/FiraCode-Medium.ttf"): Path("site/fonts/FiraCode-Medium.woff2"),
    Path("/tmp/railway-sans/fonts/TTF/Railway.ttf"): Path("site/fonts/Railway.woff2"),
    Path("/tmp/railway-sans/fonts/TTF/RailwayAlternate.ttf"): Path("site/fonts/RailwayAlternate.woff2"),
}

for src, dst in mapping.items():
    font = TTFont(str(src))
    font.flavor = "woff2"
    font.save(str(dst))
PY
```

License: SIL Open Font License 1.1 (OFL-1.1)
