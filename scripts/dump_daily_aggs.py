import psycopg2


NUM_NAVIS = 23
NUM_CHIPS = 350


def get_latest_ts(conn, agg_period, netplay_compatibility, on):
    cur = conn.cursor()
    cur.execute(
        """
        select max(ts)
        from rounds
        where
            netplay_compatibility = %s and
            date_bin(%s, ts, timestamptz '2001-01-01')::date = %s
        """,
        (netplay_compatibility, agg_period, on),
    )
    ((date,),) = cur
    return date


def get_wins(conn, agg_period, netplay_compatibility, on):
    cur = conn.cursor()
    cur.execute(
        """
        select
            winner,
            loser,
            count(*) n
        from rounds
        where
            netplay_compatibility = %s and
            date_bin(%s, ts, timestamptz '2001-01-01')::date = %s
        group by winner, loser
        order by winner, loser
        """,
        (netplay_compatibility, agg_period, on),
    )
    winrates = [[0] * NUM_NAVIS for _ in range(NUM_NAVIS)]
    for winner, loser, wins in cur:
        winrates[winner][loser] = wins
    return winrates


def get_picks(conn, agg_period, netplay_compatibility, on):
    cur = conn.cursor()
    cur.execute(
        """
        with
            picks as (
                select
                    date_bin(%s, ts, timestamptz '2001-01-01')::date period,
                    unnest(array[winner, loser]) navi,
                    count(*) n
                from rounds
                where
                    netplay_compatibility = %s and
                    date_bin(%s, ts, timestamptz '2001-01-01')::date = %s
                group by period, navi
            )
        select
            picks.navi navi,
            coalesce(picks.n, 0) picks
        from picks
        order by navi
        """,
        (agg_period, netplay_compatibility, agg_period, on),
    )
    pickrates = [0] * NUM_NAVIS
    for navi, picks in cur:
        pickrates[navi] = picks
    return pickrates


def get_turns_to_win(conn, agg_period, netplay_compatibility, on):
    cur = conn.cursor()
    cur.execute(
        """
        with
            turns as (
                select
                    date_bin(%s, ts, timestamptz '2001-01-01')::date period,
                    winner navi,
                    array_agg(turns) t
                from rounds
                where
                    netplay_compatibility = %s and
                    date_bin(%s, ts, timestamptz '2001-01-01')::date = %s
                group by period, navi
            )
        select
            navi, t
        from turns
        order by navi
        """,
        (agg_period, netplay_compatibility, agg_period, on),
    )
    turns_to_win = [[] for _ in range(NUM_NAVIS)]
    for navi, t in cur:
        turns_to_win[navi] = t
    return turns_to_win


def get_winning_chips(conn, agg_period, netplay_compatibility, on, navi):
    cur = conn.cursor()
    cur.execute(
        """
        select
            chip_id, n, wins, total
        from
        (
            select
                date_bin(%s, ts, timestamptz '2001-01-01')::date period,
                winner navi,
                chip_id,
                count(*) n,
                (
                    select
                        count(*)
                    from rounds r1
                    inner join folder_chips fc1 on
                        rounds_hash = r1.hash and
                        chip_id = folder_chips.chip_id
                    where
                        winner = rounds.winner
                ) wins,
                (
                    select
                        count(*)
                    from rounds r1
                    inner join folder_chips fc1 on
                        rounds_hash = r1.hash and
                        chip_id = folder_chips.chip_id
                    where
                        loser = rounds.winner or
                        winner = rounds.winner
                ) total
            from rounds
            inner join
                folder_chips on rounds.hash = folder_chips.rounds_hash
            where
                netplay_compatibility = %s and
                date_bin(%s, ts, timestamptz '2001-01-01')::date = %s
            group by period, navi, chip_id
        ) t
        where navi = %s
        order by chip_id
        """,
        (agg_period, netplay_compatibility, agg_period, on, navi),
    )
    winning_chips = [None] * NUM_CHIPS
    for chip_id, n, wins, total in cur:
        winning_chips[chip_id] = (n, wins, total)
    return winning_chips


import datetime
import json
import argparse
import os

agg_period = "1 day"
netplay_compatibility = "exe45_pvp_preview2_bf3"

argparser = argparse.ArgumentParser()
argparser.add_argument(
    "start_date",
    type=lambda d: datetime.datetime.strptime(d, "%Y-%m-%d").date(),
)
argparser.add_argument(
    "end_date",
    type=lambda d: datetime.datetime.strptime(d, "%Y-%m-%d").date(),
    default=datetime.date.today(),
    nargs="?",
)
argparser.add_argument(
    "--dsn",
    default="postgres://bn45pvpstats@%2Fvar%2Frun%2Fpostgresql/bn45pvpstats",
)
argparser.add_argument(
    "--out",
    default="data/daily",
)
args = argparser.parse_args()

try:
    os.makedirs(args.out)
except FileExistsError:
    pass

d = args.start_date
end = args.end_date

conn = psycopg2.connect(args.dsn)

while d <= end:
    print(d)

    with conn:
        latest_ts = get_latest_ts(conn, agg_period, netplay_compatibility, d)
        wins = get_wins(conn, agg_period, netplay_compatibility, d)
        picks = get_picks(conn, agg_period, netplay_compatibility, d)
        turns_to_win = get_turns_to_win(conn, agg_period, netplay_compatibility, d)
        winning_chips_by_navi = [
            get_winning_chips(conn, agg_period, netplay_compatibility, d, navi)
            for navi in range(NUM_NAVIS)
        ]

    with open(os.path.join(args.out, f"{d.strftime('%Y-%m-%d')}.json"), "w") as f:
        json.dump(
            {
                "latest_ts": latest_ts.isoformat() if latest_ts is not None else None,
                "wins": wins,
                "picks": picks,
                "turns_to_win": turns_to_win,
                "winning_chips_by_navi": winning_chips_by_navi,
            },
            f,
        )

    d += datetime.timedelta(days=1)
