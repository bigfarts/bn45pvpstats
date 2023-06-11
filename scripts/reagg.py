import os
import datetime
import json
import argparse

argparser = argparse.ArgumentParser()
argparser.add_argument(
    "--data-dir",
    default="data",
)
args = argparser.parse_args()


daily_data_dir = os.path.join(args.data_dir, "1day")
daily_data = {}


for fn in os.listdir(daily_data_dir):
    with open(os.path.join(daily_data_dir, fn), "r") as f:
        date, _ = os.path.splitext(fn)
        date = datetime.datetime.strptime(date, "%Y-%m-%d").date()
        d = json.load(f)
        daily_data[date] = d


def merge_latest_ts(vs):
    d = max(
        (datetime.datetime.fromisoformat(v) for v in vs if v is not None), default=None
    )
    return d.isoformat() if d is not None else None


def merge_wins(vs):
    return [[sum(row) for row in zip(*rows)] for rows in zip(*vs)]


def merge_picks(vs):
    return [sum(row) for row in zip(*vs)]


def merge_turns_to_win(vs):
    return [[cell for row in rows for cell in row] for rows in zip(*vs)]


def merge_winning_chips_by_navi(vs):
    def _merge_row(row):
        tot = None

        for col in row:
            if col is None:
                continue

            if tot is None:
                tot = col
                continue

            n1, wins1, total1 = tot
            n2, wins2, total2 = col

            return [n1 + n2, wins1 + wins2, total1 + total2]
        return tot

    return [[_merge_row(row) for row in zip(*rows)] for rows in zip(*vs)]


def merge(ds):
    return {
        "latest_ts": merge_latest_ts(d["latest_ts"] for d in ds if d is not None),
        "wins": merge_wins(d["wins"] for d in ds if d is not None),
        "picks": merge_picks(d["picks"] for d in ds if d is not None),
        "turns_to_win": merge_turns_to_win(
            d["turns_to_win"] for d in ds if d is not None
        ),
        "winning_chips_by_navi": merge_winning_chips_by_navi(
            d["winning_chips_by_navi"] for d in ds if d is not None
        ),
    }


def get_monday(date: datetime.date):
    return date - datetime.timedelta(days=date.weekday())


def date_range(start, end, delta):
    while start < end:
        yield start
        start += delta


today = datetime.date.today()


# do weekly aggregation
weekly_data_dir = os.path.join(args.data_dir, "1week")
try:
    os.makedirs(weekly_data_dir)
except FileExistsError:
    pass

d = get_monday(min(daily_data.keys()))
while d < get_monday(today) + datetime.timedelta(days=7):
    with open(os.path.join(weekly_data_dir, f"{d.isoformat()}.json"), "w") as f:
        json.dump(
            merge(
                [
                    daily_data.get(t)
                    for t in date_range(
                        d,
                        d + datetime.timedelta(days=7),
                        datetime.timedelta(days=1),
                    )
                ]
            ),
            f,
        )
    d += datetime.timedelta(days=7)

# do monthly aggregation
monthly_data_dir = os.path.join(args.data_dir, "1month")
try:
    os.makedirs(monthly_data_dir)
except FileExistsError:
    pass

d = min(daily_data.keys())
d = datetime.date(d.year, d.month, 1)
while d < datetime.date(today.year, today.month + 1, 1):
    with open(os.path.join(monthly_data_dir, f"{d.isoformat()}.json"), "w") as f:
        json.dump(
            merge(
                [
                    daily_data.get(t)
                    for t in date_range(
                        d,
                        datetime.date(d.year, d.month + 1, 1),
                        datetime.timedelta(days=1),
                    )
                ]
            ),
            f,
        )
    d = datetime.date(d.year, d.month + 1, 1)

# do 3 monthly aggregation
monthly3_data_dir = os.path.join(args.data_dir, "3month")
try:
    os.makedirs(monthly3_data_dir)
except FileExistsError:
    pass

d = min(daily_data.keys())
d = datetime.date(d.year, (d.month - 1) // 3 * 3 + 1, 1)
while d < datetime.date(today.year, today.month + 3, 1):
    with open(os.path.join(monthly3_data_dir, f"{d.isoformat()}.json"), "w") as f:
        json.dump(
            merge(
                [
                    daily_data.get(t)
                    for t in date_range(
                        d,
                        datetime.date(d.year, d.month + 3, 1),
                        datetime.timedelta(days=1),
                    )
                ]
            ),
            f,
        )
    d = datetime.date(d.year, d.month + 3, 1)

# do all time aggregation
with open(os.path.join(args.data_dir, f"alltime.json"), "w") as f:
    json.dump(merge(list(daily_data.values())), f)
