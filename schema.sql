create table rounds
( hash bytea primary key
, ts timestamptz not null
, turns integer not null
, winner integer not null
, loser integer not null
, netplay_compatibility text not null
);

create table folder_chips
( rounds_hash bytea not null foreign key references rounds (hash)
, is_winner boolean not null
, idx integer not null
, chip_id integer not null
, chip_code char not null
, is_regchip boolean not null
, primary key (rounds_hash, is_winner, idx)
);

create index on rounds(winner, netplay_compatibility);
create index on rounds(loser, netplay_compatibility);
create index on rounds(ts, netplay_compatibility);
create index on rounds(netplay_compatibility);
create index on folder_chips(rounds_hash, chip_id, is_winner);
