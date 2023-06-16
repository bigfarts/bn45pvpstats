<!doctype html>
<html lang="${LANG}">
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet" integrity="sha384-9ndCyUaIbzAi2FUVXJi0CjmCapSmO7SnpJef0486qhLnuZ2cdeRhO02iuK6FUUVM" crossorigin="anonymous">
        <script src="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/js/bootstrap.bundle.min.js" integrity="sha384-geWF76RCwLtnZ8qwWowPQNguL3RmwHVBC9FhGdlKrxdiJJigb/j/68SIy3Te4Bkz" crossorigin="anonymous"></script>
        <style>
        html {
            height: 100%;
        }

        body {
            height: 100%;
        }
        </style>
        <title>${LOCALE["common"]["title"]}</title>
    </head>
    <body>
        <main class="d-flex flex-nowrap">
            <div
                class="d-flex flex-column flex-shrink-0 sticky-top"
                style="width: 280px"
            >
                <div class="d-flex flex-row flex-grow-0 p-3">
                    <h1 class="fs-5 fw-semibold m-0">
                        <a
                            href="/${LANG}/summary/${agg_period}"
                            class="d-flex flex-shrink-0 link-body-emphasis text-decoration-none"
                        >
                            <span>${LOCALE["common"]["title"]}</span>
                        </a>
                    </h1>
                    % for lang, name in [ \
                        ("en", "EN"), \
                        ("ja", "日本語"), \
                    ]:
                    <a href="/${lang}/${REL_URL}" class="mx-1 ${"d-none" if LANG == lang else ""}" lang="${lang}">${name}</a>
                    % endfor
                </div>
                <ul class="nav nav-pills flex-column flex-nowrap overflow-auto">
                    % for i, name in ((i, v) for i, v in enumerate(NAVIS) if v is not None):
                    <li class="nav-item px-2">
                        <a href="/${LANG}/navis/${name}/${agg_period}" class="px-2 nav-link ${"active" if current_navi == i else "link-body-emphasis"}">
                            <img src="https://www.therockmanexezone.com/pages/exe45-pvp-patch/img/navi_${name}.png" class="pe-none me-2" style="image-rendering: pixelated" height="48" width="40">
                            ${LOCALE["common"]["navis"][i]}
                        </a>
                    </li>
                    % endfor
                </ul>
            </div>
            <div class="flex-grow-1 flex-shrink-1 d-flex" style="min-width: 0">
                ${self.body(NAVIS=NAVIS, LOCALE=LOCALE)}
            </div>
        </main>
        <script>
        const tooltipTriggerList = [].slice.call(document.querySelectorAll('[data-bs-toggle="tooltip"]'))
        const tooltipList = tooltipTriggerList.map(function (tooltipTriggerEl) {
            return new bootstrap.Tooltip(tooltipTriggerEl)
        });
        </script>
    </body>
</html>
