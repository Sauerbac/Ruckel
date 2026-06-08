# Bundled fonts

Ruckel ships these fonts as local files (no CDN — it is an offline desktop app,
see ADR-0018). Both families are licensed under the **SIL Open Font License 1.1**,
which permits bundling and redistribution.

| File | Family | Upstream | License |
|---|---|---|---|
| `space-grotesk-variable.woff2` | Space Grotesk (variable, latin, weight axis 300–700) | https://github.com/floriankarsten/space-grotesk | OFL-1.1 |
| `ibm-plex-mono-400.woff2` | IBM Plex Mono 400 (latin) | https://github.com/IBM/plex | OFL-1.1 |
| `ibm-plex-mono-500.woff2` | IBM Plex Mono 500 (latin) | https://github.com/IBM/plex | OFL-1.1 |
| `ibm-plex-mono-600.woff2` | IBM Plex Mono 600 (latin) | https://github.com/IBM/plex | OFL-1.1 |

Space Grotesk is the `latin-wght-normal` variable subset; IBM Plex Mono is static
on Fontsource, so the three weights the design system uses (400/500/600) are
bundled individually. All `.woff2` files are the `latin` subsets distributed by
[Fontsource](https://fontsource.org/) via jsDelivr. Full OFL text:
https://openfontlicense.org/
