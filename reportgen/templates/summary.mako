<%inherit file="_layout.mako" />
<%page args="NAVIS,LOCALE" />
<%
import json
import itertools
import statistics
from palettable.cartocolors.sequential import TealGrn_7, RedOr_7, PurpOr_7
%>

<%
def get_ranking(row, all_picks, all_turns_to_win):
    ranking = []
    for i, w in enumerate(row):
        if NAVIS[i] is None:
            continue
        wins = sum(w)
        losses = sum(list(zip(*row))[i])
        total = wins + losses

        picks = all_picks[i] if all_picks else 0
        turns_to_win = all_turns_to_win[i]

        total_picks = sum(all_picks)
        max_picks = max(all_picks)

        ranking.append((i, wins, total, picks, total_picks, max_picks, turns_to_win))
    ranking.sort(key=lambda kv: kv[3] / kv[4] if kv[4] != 0 else float('-inf'), reverse=True)
    return ranking
%>

<div class="d-flex flex-grow-1 flex-shrink-1 flex-column" style="min-width: 0">
    <ul class="nav nav-tabs flex-shrink-0">
    % for name in ["alltime", "3month", "1month", "1week"]:
        <li class="nav-item">
            <a class="nav-link${" active" if name == agg_period else ""}" href="/${LANG}/summary/${name}">${LOCALE["common"]["agg_periods"][name]}</a>
        </li>
    % endfor
    </ul>

    <div class="flex-grow-1 flex-shrink-0">
        <table class="table table-striped table-hover border-start" style="width: max-content">
            <thead class="sticky-top">
                <tr>
                    % for date, _ in data:
                    <th colspan="4" class="text-center border-end">${date.strftime("%Y-%m-%d")}</th>
                    % endfor
                </tr>
                <tr>
                    % for _, _ in data:
                    <th style="width: 200px"></th>
                    <th style="width: 150px">${LOCALE["common"]["stats"]["picks"]}</th>
                    <th style="width: 150px">${LOCALE["common"]["stats"]["wins"]}</th>
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
                        *(get_ranking(tab["wins"], tab["picks"], tab["turns_to_win"]) for _, tab in data),
                        fillvalue=None,
                    ))
                ]
                %>
                % for row in rankings_t:
                <tr>
                    % for colno, (i, wins, total, picks, total_picks, max_picks, turns_to_win) in enumerate(row):
                    <%
                        navi = NAVIS[i]
                        name = LOCALE["common"]["navis"][i]
                    %>
                    <td class="align-middle${" table-secondary" if picks == 0 else ""}">
                        <a href="/${LANG}/navis/${navi}/${agg_period}" class="d-flex align-items-center">
                            <img src="https://www.therockmanexezone.com/pages/exe45-pvp-patch/img/navi_${navi}.png" alt="${name}" style="image-rendering: pixelated" class="d-block me-2"> ${name}
                        </a>
                    </td>
                    % if total != 0:
                    <%
                        winrate = wins / total
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
                        <div><small>${picks}/${total_picks} (${f'{pickrate:.2f}'})</small></div>
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
                    <td class="text-center align-middle${" table-secondary" if picks == 0 else ""}" colspan="3">
                        ${LOCALE["common"]["no-data"]}
                    </td>
                    % endif
                    % endfor
                </tr>
                % endfor
            </tbody>
        </table>
    </div>
</div>
