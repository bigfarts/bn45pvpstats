import datetime
import toml
import os
import json
import itertools
import shutil
from mako.lookup import TemplateLookup

data_dir = "data"
out_dir = "report"

shutil.rmtree(out_dir)

aggregated_data = {}

for agg_period in ["1week", "1month", "3month", "alltime"]:
    entries = []

    d = os.path.join(data_dir, agg_period)
    for fn in os.listdir(d):
        date, _ = os.path.splitext(fn)
        date = datetime.datetime.strptime(date, "%Y-%m-%d").date()

        with open(os.path.join(d, fn), "r") as f:
            data = json.load(f)

        entries.append((date, data))

    entries.sort(key=lambda dv: dv[0])
    entries = list(itertools.dropwhile(lambda dd: dd[1]["latest_ts"] is None, entries))
    entries.reverse()

    aggregated_data[agg_period] = entries


lookup = TemplateLookup(
    directories=[os.path.join(os.path.dirname(__file__), "templates")],
)


def open_with_makedirs(fn, *args, **kwargs):
    try:
        os.makedirs(os.path.dirname(fn))
    except FileExistsError:
        pass
    return open(fn, *args, **kwargs)


def render_to_file(tmpl, fn, **kwargs):
    print(fn)
    with open_with_makedirs(os.path.join(out_dir, fn), "w") as f:
        f.write(tmpl.render(**kwargs))


def copy_file(src, fn, **kwargs):
    print(fn)
    with open_with_makedirs(os.path.join(out_dir, fn), "w") as f, open(
        os.path.join(out_dir, src), "r"
    ) as r:
        shutil.copyfileobj(r, f)


NAVIS = [
    "megaman",
    "roll",
    "gutsman",
    "windman",
    "searchman",
    "fireman",
    "thunderman",
    "protoman",
    "numberman",
    "metalman",
    "junkman",
    "aquaman",
    "woodman",
    None,
    None,
    "starman",
    "shadowman",
    "knightman",
    "napalmman",
    "iceman",
    "elecman",
    "plantman",
    "bass",
]

shutil.copytree(
    os.path.join(os.path.dirname(__file__), "images"),
    os.path.join(out_dir, "images"),
)

try:
    os.remove(os.path.join(out_dir, "images", ".gitignore"))
except FileNotFoundError:
    pass

for lang in ["en", "ja"]:
    locale = {}

    locale_dir = os.path.join(os.path.dirname(__file__), "locales", lang)

    for fn in os.listdir(locale_dir):
        ns, ext = os.path.splitext(fn)
        if ext != ".toml":
            continue

        with open(os.path.join(locale_dir, fn), "r") as f:
            locale[ns] = toml.load(f)

    summary_tmpl = lookup.get_template("summary.mako")
    navi_tmpl = lookup.get_template("navi.mako")
    for agg_period, data in aggregated_data.items():
        render_to_file(
            summary_tmpl,
            f"{lang}/summary/{agg_period}/index.html",
            NAVIS=NAVIS,
            current_navi=None,
            data=data,
            agg_period=agg_period,
            REL_URL=f"summary/{agg_period}",
            LANG=lang,
            LOCALE=locale,
        )

        for i, name in ((i, v) for i, v in enumerate(NAVIS) if v is not None):
            render_to_file(
                navi_tmpl,
                f"{lang}/navis/{name}/{agg_period}/index.html",
                NAVIS=NAVIS,
                current_navi=i,
                data=data,
                agg_period=agg_period,
                REL_URL=f"navis/{name}/{agg_period}",
                LANG=lang,
                LOCALE=locale,
            )

    copy_file(f"{lang}/summary/alltime/index.html", f"{lang}/index.html")
