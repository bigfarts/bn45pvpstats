<%inherit file="_layout.mako" />
<%page args="NAVIS,LOCALE" />
<%
import json
import itertools
import statistics
from palettable.cartocolors.sequential import TealGrn_7, RedOr_7, PurpOr_7
%>

<%
def get_chips_ranking(winning_chips, picks):
    ranking = []
    for i, (wins, losses) in enumerate(winning_chips):
        total = wins + losses
        ranking.append((i, wins, total, picks, max(wins + losses for wins, losses in winning_chips)))
    ranking.sort(key=lambda kv: kv[2] / kv[3] if kv[3] != 0 else float('-inf'), reverse=True)
    return ranking
%>

<div class="d-flex flex-grow-1 flex-shrink-1 flex-column" style="min-width: 0">
    <h2 class="mx-1 my-2 fs-5">${LOCALE["common"]["facets"]["picks-and-wins"]}</h2>
    <ul class="nav nav-tabs flex-shrink-0">
    % for name in ["1week", "1month", "3month", "alltime"]:
        <li class="nav-item">
            <a class="nav-link${" active" if name == agg_period else ""}" href="/${LANG}/navis/${NAVIS[current_navi]}/${name}">${LOCALE["common"]["agg_periods"][name]}</a>
        </li>
    % endfor
    </ul>

    <div class="flex-shrink-0">
        <table class="table table-striped table-hover border-start" style="width: max-content">
            <thead class="sticky-top">
                <tr>
                    % for date, _ in data:
                    <th colspan="4" class="text-center border-end">${date.strftime("%Y-%m-%d")}</th>
                    % endfor
                </tr>
                <tr>
                    % for _, _ in data:
                    <th style="width: 72.5px"></th>
                    <th style="width: 150px">${LOCALE["common"]["stats"]["picks"]}</th>
                    <th style="width: 150px">${LOCALE["common"]["stats"]["wins"]}</th>
                    <th style="width: 150px" class="border-end">${LOCALE["common"]["stats"]["turns-to-win"]}</th>
                    % endfor
                </tr>
            </thead>
            <tbody>
                <tr>
                    % for date, tab in data:
                    <%
                        wins = sum(tab["wins"][current_navi]) if tab["wins"] else 0
                        picks = tab["picks"][current_navi] if tab["picks"] else 0
                        median_turns_to_win = statistics.median(tab["turns_to_win"][current_navi]) if tab["turns_to_win"][current_navi] else 0

                        losses = sum(tab["wins"][i][current_navi] for i in range(len(NAVIS))) if tab["wins"] else 0
                        total = wins + losses
                        total_picks = sum(tab["picks"])
                        max_picks = max(tab["picks"], default=0)

                        winrate = wins / total if total != 0 else None
                        pickrate = picks / total_picks if total_picks != 0 else 0

                        rel_winrate = winrate
                        rel_pickrate = picks / max_picks if max_picks != 0 else 0
                        rel_median_turns_to_win = median_turns_to_win / 15.0

                        win_color = TealGrn_7.colors[round(rel_winrate * (len(TealGrn_7.colors) - 1))] if rel_winrate is not None else None
                        pick_color = RedOr_7.colors[round(rel_pickrate * (len(RedOr_7.colors) - 1))]
                        median_turns_to_win_color = PurpOr_7.colors[round(rel_median_turns_to_win * (len(PurpOr_7.colors) - 1))]
                    %>
                    % if total != 0:
                    <td></td>
                    <td class="align-middle">
                        <div><small>${picks}/${total_picks} (${f'{pickrate:.2f}'})</small></div>
                        <div style="width: 100%; height: 5px">
                            <div style="background-color: ${f"rgb({pick_color[0]}, {pick_color[1]}, {pick_color[2]})"}; width: ${rel_pickrate * 100}%; height: 100%"></div>
                        </div>
                    </td>
                    <td class="align-middle">
                        <div><small>${wins}/${total} (${f'{winrate:.2f}'})</small></div>
                        <div style="width: 100%; height: 5px">
                            <div style="background-color: ${f"rgb({win_color[0]}, {win_color[1]}, {win_color[2]})" if win_color is not None else "0, 0, 0"}; width: ${rel_winrate * 100 if winrate is not None else 0}%; height: 100%"></div>
                        </div>
                    </td>
                    <td class="align-middle border-end">
                        <div><small>${f'{median_turns_to_win:.0f}'}</small></div>
                        <div style="width: 100%; height: 5px">
                            <div style="background-color: ${f"rgb({median_turns_to_win_color[0]}, {median_turns_to_win_color[1]}, {median_turns_to_win_color[2]})"}; width: ${rel_median_turns_to_win * 100}%; height: 100%"></div>
                        </div>
                    </td>
                    % else:
                    <td class="text-center align-middle${" table-secondary" if picks == 0 else ""}" colspan="4">
                        ${LOCALE["common"]["no-data"]}
                    </td>
                    % endif
                    % endfor
                </tr>
            </tbody>
        </table>
    </div>

    <h2 class="mx-1 my-2 fs-5">${LOCALE["common"]["facets"]["chip-usage"]}</h2>
    <ul class="nav nav-tabs flex-shrink-0">
        % for name in ["1week", "1month", "3month", "alltime"]:
            <li class="nav-item">
                <a class="nav-link${" active" if name == agg_period else ""}" href="/${LANG}/navis/${NAVIS[current_navi]}/${name}">${LOCALE["common"]["agg_periods"][name]}</a>
            </li>
        % endfor
    </ul>
    <div class="flex-shrink-0">
        <table class="table table-striped table-hover border-start" style="width: max-content">
            <thead class="sticky-top">
                <tr>
                    % for date, _ in data:
                    <th colspan="4" class="text-center border-end">${date.strftime("%Y-%m-%d")}</th>
                    % endfor
                </tr>
                <tr>
                    % for _, _ in data:
                    <th style="width: 72.5px"></th>
                    <th style="width: 150px">${LOCALE["common"]["stats"]["picks"]}</th>
                    <th style="width: 150px">${LOCALE["common"]["stats"]["wins"]}</th>
                    <th style="width: 150px" class="border-end"></th>
                    % endfor
                </tr>
            </thead>
            <tbody>
                <%
                chips_t = list(itertools.zip_longest(
                    *(get_chips_ranking(tab["chips"][current_navi], tab["picks"][current_navi] if tab["picks"] else 0) for _, tab in data),
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
                        <span title="${LOCALE["chips"]["names"][chip_id]}" data-bs-toggle="tooltip" data-bs-placement="right">
                            <img src="/images/chips/${chip_id}_full.png" alt="${LOCALE["chips"]["names"][chip_id]}" style="image-rendering: pixelated" width="56" height="48">
                        </span>
                    </td>
                    <td class="align-middle">
                        <div><small>${total}/${picks} (${f'{pickrate:.2f}'})</small></div>
                        <div style="width: 100%; height: 5px">
                            <div style="background-color: ${f"rgb({pick_color[0]}, {pick_color[1]}, {pick_color[2]})" if pick_color is not None else "0, 0, 0"}; width: ${rel_pickrate * 100}%; height: 100%"></div>
                        </div>
                    </td>
                    <td class="align-middle">
                        <div><small>${wins}/${total} (${f'{winrate:.2f}'})</small></div>
                        <div style="width: 100%; height: 5px">
                            <div style="background-color: ${f"rgb({win_color[0]}, {win_color[1]}, {win_color[2]})" if win_color is not None else "0, 0, 0"}; width: ${rel_winrate * 100 if winrate is not None else 0}%; height: 100%"></div>
                        </div>
                    </td>
                    <td class="border-end"></td>
                    % else:
                    <td class="border-end${" table-secondary" if total == 0 else ""}" colspan="4"></td>
                    % endif
                    % endfor
                </tr>
                % endfor
            </tbody>
        </table>
    </div>
</div>
