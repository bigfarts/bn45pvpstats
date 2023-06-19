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
            date_bin(%s, ts, timestamptz '2001-01-01')::date = %s and
            winner != loser
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
                    unnest(array[winner, loser]) navi,
                    count(*) n
                from rounds
                where
                    netplay_compatibility = %s and
                    date_bin(%s, ts, timestamptz '2001-01-01')::date = %s and
                    winner != loser
                group by navi
            )
        select
            picks.navi navi,
            coalesce(picks.n, 0) picks
        from picks
        order by navi
        """,
        (netplay_compatibility, agg_period, on),
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
                    winner navi,
                    array_agg(turns) t
                from rounds
                where
                    netplay_compatibility = %s and
                    date_bin(%s, ts, timestamptz '2001-01-01')::date = %s and
                    winner != loser
                group by navi
            )
        select
            navi, t
        from turns
        order by navi
        """,
        (netplay_compatibility, agg_period, on),
    )
    turns_to_win = [[] for _ in range(NUM_NAVIS)]
    for navi, t in cur:
        turns_to_win[navi] = t
    return turns_to_win


def get_chips(conn, agg_period, netplay_compatibility, on, navi):
    cur = conn.cursor()
    cur.execute(
        """
        with
            selected_rounds as (
                select
                    hash,
                    winner,
                    loser
                from rounds
                where
                    netplay_compatibility = %s and
                    date_bin(%s, ts, timestamptz '2001-01-01')::date = %s and
                    %s = any(array[rounds.winner, rounds.loser]) and
                    winner != loser
            )
        select
            chip_id,
            (
                select
                    count(*)
                from selected_rounds
                where
                    exists (
                        select *
                        from chip_uses fc2
                        where
                            fc2.rounds_hash = selected_rounds.hash and
                            fc2.chip_id = fc1.chip_id and
                            fc2.is_winner
                        ) and
                    winner = %s
            ) wins,
            (
                select
                    count(*)
                from selected_rounds
                where
                    exists (
                        select *
                        from chip_uses fc2
                        where
                            fc2.rounds_hash = selected_rounds.hash and
                            fc2.chip_id = fc1.chip_id and
                            not fc2.is_winner
                        ) and
                    loser = %s
            ) losses
        from chip_uses fc1
        group by chip_id
        """,
        (netplay_compatibility, agg_period, on, navi, navi, navi),
    )
    winning_chips = [[0, 0] for _ in range(NUM_CHIPS)]
    for chip_id, wins, losses in cur:
        if chip_id >= len(winning_chips):
            continue
        winning_chips[chip_id] = (wins, losses)
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
    "--data-dir",
    default="data",
)
args = argparser.parse_args()

out = os.path.join(args.data_dir, "1day")
try:
    os.makedirs(out)
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
        chips = [
            get_chips(conn, agg_period, netplay_compatibility, d, navi)
            for navi in range(NUM_NAVIS)
        ]

    with open(os.path.join(out, f"{d.strftime('%Y-%m-%d')}.json"), "w") as f:
        json.dump(
            {
                "latest_ts": latest_ts.isoformat() if latest_ts is not None else None,
                "wins": wins,
                "picks": picks,
                "turns_to_win": turns_to_win,
                "chips": chips,
            },
            f,
        )

    d += datetime.timedelta(days=1)
