<%inherit file="_layout.mako" />
<%page args="NAVIS,LOCALE" />
<%
import json
import itertools
import statistics
from palettable.cartocolors.sequential import TealGrn_7, RedOr_7, PurpOr_7
%>

<%
def get_chips_ranking(current_navi, winning_chips, all_wins):
    ranking = []
    for i, (win, loss) in enumerate(winning_chips):
        total = win + loss

        navi_win = sum(all_wins[current_navi])
        navi_loss = sum(list(zip(*all_wins))[current_navi])

        ranking.append((i, win, total, navi_win + navi_loss, max(win + loss for wins, loss in winning_chips)))
    ranking.sort(key=lambda kv: kv[2], reverse=True)
    return ranking


def get_h2h_ranking(current_navi, all_wins, all_turns_to_win):
    ranking = []

    picks = [all_wins[current_navi][i] + all_wins[i][current_navi] for i in range(len(NAVIS))]
    total_picks = sum(picks)
    max_picks = max(picks, default=0)

    for i in range(len(NAVIS)):
        if NAVIS[i] is None:
            continue
        win = all_wins[current_navi][i]
        loss = all_wins[i][current_navi]
        turns_to_win = all_turns_to_win[current_navi][i]
        ranking.append((i, win, win + loss, total_picks, max_picks, turns_to_win))
    ranking.sort(key=lambda kv: kv[2], reverse=True)
    return ranking
%>

<div class="d-flex flex-grow-1 flex-shrink-1 flex-column" style="min-width: 0">
    <h2 class="mx-1 my-2 fs-5">${LOCALE["common"]["facets"]["head-to-head"]}</h2>
    <ul class="nav nav-tabs flex-shrink-0">
    % for name in ["alltime", "3month", "1month", "1week"]:
        <li class="nav-item">
            <a class="nav-link${" active" if name == agg_period else ""}" href="/${LANG}/navis/${NAVIS[current_navi]}/${name}">${LOCALE["common"]["agg-periods"][name]}</a>
        </li>
    % endfor
    </ul>

    <div class="flex-shrink-0">
        <table class="table table-striped table-hover border-start" style="width: max-content">
            <thead class="sticky-top">
                <tr>
                    % for date, _ in data:
                    <th colspan="3" class="text-center border-end">${date.strftime("%Y-%m-%d")}</th>
                    % endfor
                </tr>
                <tr>
                    % for _, _ in data:
                    <th style="width: 200px"></th>
                    <th style="width: 200px">${LOCALE["common"]["stats"]["picks-and-wins"]}</th>
                    <th style="width: 128px" class="border-end">${LOCALE["common"]["stats"]["turns-to-win"]}</th>
                    % endfor
                </tr>
            </thead>
            <tbody>
                <%
                legal_navis = [i for i, v in enumerate(NAVIS) if v is not None]

                rankings_t = [
                    [v if v is not None else [legal_navis[i], 0, 0, 0, 0, 0] for v in vs]
                    for i, vs in enumerate(itertools.zip_longest(
                        *(get_h2h_ranking(current_navi, tab["wins"], tab["turns_to_win"]) for _, tab in data),
                        fillvalue=None,
                    ))
                ]
                %>
                % for row in rankings_t:
                <tr>
                    % for colno, (i, wins, picks, total_picks, max_picks, turns_to_win) in enumerate(row):
                    <%
                        navi = NAVIS[i]
                        name = LOCALE["common"]["navis"][i]
                    %>
                    <td class="align-middle${" table-secondary" if picks == 0 else ""}">
                        <a href="/${LANG}/navis/${navi}/${agg_period}" class="d-flex align-items-center">
                            <img src="https://www.therockmanexezone.com/pages/exe45-pvp-patch/img/navi_${navi}.png" alt="${name}" style="image-rendering: pixelated" class="d-block me-2">
                            <span class="name">${name}</span>
                        </a>
                    </td>
                    % if picks != 0:
                    <%
                        winrate = wins / picks
                        pickrate = picks / total_picks

                        rel_winrate = winrate
                        rel_pickrate = picks / max_picks

                        turns_to_win_counts = [0] * 16
                        for v in turns_to_win:
                            turns_to_win_counts[v] += 1
                        max_turns_to_win_count = max(turns_to_win_counts)

                        win_color = TealGrn_7.colors[round(rel_winrate * (len(TealGrn_7.colors) - 1))] if rel_winrate is not None else None
                        pick_color = RedOr_7.colors[round(rel_pickrate * (len(RedOr_7.colors) - 1))] if rel_pickrate is not None else None
                    %>
                    <td class="align-middle">
                        <div><small>${wins}/${picks}/${total_picks} (${f'{winrate:.2f}'}/${f'{pickrate:.2f}'})</small></div>
                        <div style="width: 100%; height: 5px">
                            <div style="background-color: ${f"rgb({pick_color[0]}, {pick_color[1]}, {pick_color[2]})" if pick_color is not None else "0, 0, 0"}; width: ${rel_pickrate * 100}%; height: 100%">
                                <div style="background-color: ${f"rgb({win_color[0]}, {win_color[1]}, {win_color[2]})" if win_color is not None else "0, 0, 0"}; width: ${rel_winrate * 100 if winrate is not None else 0}%; height: 100%"></div>
                            </div>
                        </div>
                    </td>
                    <td class="align-middle border-end">
                        <div class="d-flex flex-row align-items-end" style="height: 48px">
                            % for i, v in enumerate(turns_to_win_counts):
                            <%
                            rel_v = v / max_turns_to_win_count if max_turns_to_win_count != 0 else 0
                            rate = v / sum(turns_to_win_counts) if sum(turns_to_win_counts) != 0 else 0
                            v_color = PurpOr_7.colors[round(rel_v * (len(PurpOr_7.colors) - 1))]
                            %>
                            <div style="height: ${rel_v * 100}%; width: 5px; background-color: ${f"rgb({v_color[0]}, {v_color[1]}, {v_color[2]})"}; margin-right: 2px" title="${LOCALE["common"]["stats"]["turns-to-win-hint"].format(turns=i, count=v, rate=rate)}" data-bs-toggle="tooltip" data-bs-placement="top"></div>
                            % endfor
                        </div>
                    </td>
                    % else:
                    <td class="text-center align-middle${" table-secondary" if picks == 0 else ""}" colspan="2">
                        ${LOCALE["common"]["no-data"]}
                    </td>
                    % endif
                    % endfor
                </tr>
                % endfor
            </tbody>
        </table>
    </div>

    <h2 class="mx-1 my-2 fs-5">${LOCALE["common"]["facets"]["chip-usage"]}</h2>
    <ul class="nav nav-tabs flex-shrink-0">
        % for name in ["alltime", "3month", "1month", "1week"]:
            <li class="nav-item">
                <a class="nav-link${" active" if name == agg_period else ""}" href="/${LANG}/navis/${NAVIS[current_navi]}/${name}">${LOCALE["common"]["agg-periods"][name]}</a>
            </li>
        % endfor
    </ul>
    <div class="flex-shrink-0">
        <table class="table table-striped table-hover border-start" style="width: max-content">
            <thead class="sticky-top">
                <tr>
                    % for date, _ in data:
                    <th colspan="3" class="text-center border-end">${date.strftime("%Y-%m-%d")}</th>
                    % endfor
                </tr>
                <tr>
                    % for _, _ in data:
                    <th style="width: 200px"></th>
                    <th style="width: 200px">${LOCALE["common"]["stats"]["picks-and-wins"]}</th>
                    <th style="width: 128px" class="border-end"></th>
                    % endfor
                </tr>
            </thead>
            <tbody>
                <%
                chips_t = list(itertools.zip_longest(
                    *(get_chips_ranking(current_navi, tab["chips"][current_navi], tab["wins"]) for _, tab in data),
                    fillvalue=None,
                ))
                %>
                % for row in itertools.takewhile(lambda row: any(cols[2] != 0 for cols in row), chips_t):
                <tr>
                    % for chip_id, wins, total, picks, max_picks in row:
                    % if total != 0:
                    <%
                        winrate = wins / total
                        pickrate = total / picks

                        rel_winrate = winrate
                        rel_pickrate = total / max_picks if max_picks != 0 else 0

                        win_color = TealGrn_7.colors[round(rel_winrate * (len(TealGrn_7.colors) - 1))] if rel_winrate is not None else None
                        pick_color = RedOr_7.colors[round(rel_pickrate * (len(RedOr_7.colors) - 1))] if rel_pickrate is not None else None
                    %>
                    <td>
                        <div class="d-flex align-items-center">
                            <img src="/images/chips/${chip_id}_full.png" alt="${LOCALE["chips"]["names"][chip_id]}" style="image-rendering: pixelated" class="d-block me-2" width="56" height="48">
                            <span class="name">${LOCALE["chips"]["names"][chip_id]}</span>
                        </div>
                    </td>
                    <td class="align-middle">
                        <div><small>${wins}/${total}/${picks} (${f'{winrate:.2f}'}/${f'{pickrate:.2f}'})</small></div>
                        <div style="width: 100%; height: 5px">
                            <div style="background-color: ${f"rgb({pick_color[0]}, {pick_color[1]}, {pick_color[2]})" if pick_color is not None else "0, 0, 0"}; width: ${rel_pickrate * 100}%; height: 100%">
                                <div style="background-color: ${f"rgb({win_color[0]}, {win_color[1]}, {win_color[2]})" if win_color is not None else "0, 0, 0"}; width: ${rel_winrate * 100 if winrate is not None else 0}%; height: 100%"></div>
                            </div>
                        </div>
                    </td>
                    <td class="border-end"></td>
                    % else:
                    <td class="border-end${" table-secondary" if total == 0 else ""}" colspan="3"></td>
                    % endif
                    % endfor
                </tr>
                % endfor
            </tbody>
        </table>
    </div>
</div>
