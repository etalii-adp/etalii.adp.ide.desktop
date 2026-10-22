"""Places the influence anchors of a .ghg example: python spread-anchors.py <file.ghg>...

Every drawn influence keeps its phases and edges; only its from-at and to-at change, and every other
byte of the file is kept. The rule, which GhgExampleInfluencePhaseTests holds every example to:

- An influence arrives no earlier than it leaves: the date under its To end is at or after the date
  under its From end. Both start at one moment the two phases share; where they share none, the
  cause's end sits late in its phase and the effect's early in its own, and the two are not drawn together.
- The ends sharing one edge of one phase spread evenly across it, in date order, keeping clear of
  the phase's ends. An end alone on its edge leans only gently to the middle, so its dates decide.
  Where the first rule leaves an edge's ends bunched, its first end goes as early and its last as
  late as each one's partner allows, the rest evenly between.
- Rounding to two decimals goes towards each other, the cause down and the effect up, so it never
  breaks the first rule.

An influence from a trigger has no From end: the trigger's date is where it leaves, and only its To
end is placed - no earlier than that date, and spread with the other ends on its edge.

Run it again after adding trends, triggers or influences to an example; an influence on a hidden phase
is not drawn and is left as written.
"""
import re, sys, math

PH = ["peak", "trough", "slope", "plateau"]
KEYS = ["peak-end", "trough-end", "slope-end"]
LO, HI = 0.06, 0.94


def month(s):
    s = s.strip()
    neg = s.startswith("-")
    y, mo = s.lstrip("-").rsplit("-", 1)
    y = -int(y) if neg else int(y)
    return y * 12 + int(mo) - 1


def rnd(x):
    return int(math.floor(abs(x) + 0.5)) * (1 if x >= 0 else -1)


def boundaries(start, stop, phases, dragged):
    inner = max(1, min(phases, 4)) - 1
    res = [0] * inner
    ai, av = -1, start
    for i in range(inner + 1):
        stored = dragged[i] if i < inner and i < len(dragged) else None
        if i < inner and stored is None:
            continue
        v = stop if i == inner else min(max(stored, av), stop)
        for b in range(ai + 1, i):
            res[b] = rnd(av + (v - av) * (b - ai) / (i - ai))
        if i < inner:
            res[i] = v
        ai, av = i, v
    return [start] + res + [stop]


def parse(text):
    lines = text.split("\n")
    trends, triggers, infl, cur, sect = {}, {}, [], None, None
    for n, line in enumerate(lines):
        m = re.match(r"^(trends|triggers|notes|influences):", line)
        if m:
            sect = m.group(1); continue
        m = re.match(r"^  - id: (\S+)", line)
        if m:
            cur = {"id": m.group(1)}
            if sect == "trends":
                trends[cur["id"]] = cur
            elif sect == "triggers":
                triggers[cur["id"]] = cur
            elif sect == "influences":
                infl.append(cur)
            continue
        m = re.match(r"^    ([\w-]+): (.*?)\s*$", line)
        if m and cur is not None:
            cur[m.group(1)] = m.group(2)
            cur[m.group(1) + "@"] = n
    return lines, trends, triggers, infl


def span(t, phase):
    p = int(t.get("phases", 4))
    if phase >= p:
        return None
    b = boundaries(month(t["start"]), month(t["stop"]), p, [month(t[k]) if k in t else None for k in KEYS])
    return b[phase], b[phase + 1]


def frac(s, when):
    a, b = s
    return (when - a) / max(1, b - a)


def when_at(s, f):
    a, b = s
    return a + f * (b - a)


def run(text):
    lines, trends, triggers, infl = parse(text)
    work = []
    for i, x in enumerate(infl):
        if x["from"] in triggers:
            # A trigger is a moment: it leaves at its date, and the effect is felt no earlier.
            d = month(triggers[x["from"]]["date"])
            ts = span(trends[x["to"]], PH.index(x["to-phase"]))
            if ts is None:
                continue
            m = 0.04
            t = (ts[0] + m * (ts[1] - ts[0]), ts[1] - m * (ts[1] - ts[0]))
            t = (min(max(t[0], d), t[1]), t[1])
            work.append({"i": i, "trigger": True, "fs": (d, d + 1), "ts": ts, "f": (d, d), "t": t,
                         "meet": True, "T1": d, "T2": t[0]})
            continue
        fs = span(trends[x["from"]], PH.index(x["from-phase"]))
        ts = span(trends[x["to"]], PH.index(x["to-phase"]))
        if fs is None or ts is None:
            continue  # a hidden end is not drawn; leave it as written
        # Each end keeps clear of its phase's own ends; the effect never comes before the cause.
        m = 0.04
        w = {"i": i, "fs": fs, "ts": ts,
             "f": (fs[0] + m * (fs[1] - fs[0]), fs[1] - m * (fs[1] - fs[0])),
             "t": (ts[0] + m * (ts[1] - ts[0]), ts[1] - m * (ts[1] - ts[0]))}
        lo, hi = max(w["f"][0], w["t"][0]), min(w["f"][1], w["t"][1])
        mid = (lo + hi) / 2 if lo < hi else None
        # Start both at one shared moment where the phases meet; else late on the cause, early on the effect.
        w["meet"] = mid is not None
        if not w["meet"]:
            # Phases that never meet: the cause acts from late in its phase, the effect is felt early in its own.
            w["f"] = (fs[0] + 0.6 * (fs[1] - fs[0]), w["f"][1])
            w["t"] = (w["t"][0], ts[0] + 0.4 * (ts[1] - ts[0]))
        w["T1"] = mid if mid is not None else w["f"][1]
        w["T2"] = mid if mid is not None else w["t"][0]
        work.append(w)

    slots = {}
    for w in work:
        x = infl[w["i"]]
        if not w.get("trigger"):
            slots.setdefault((x["from"], x["from-phase"], x["from-edge"]), []).append((w, "from"))
        slots.setdefault((x["to"], x["to-phase"], x["to-edge"]), []).append((w, "to"))

    def f_of(w, side):
        return frac(w["fs"], w["T1"]) if side == "from" else frac(w["ts"], w["T2"])

    def clamp(v, r):
        return min(max(v, r[0]), r[1])

    for _ in range(300):
        for members in slots.values():
            # In date order, then evenly across the phase.
            members.sort(key=lambda mm: (f_of(*mm), mm[0]["T1"], mm[0]["T2"]))
            n = len(members)
            # Alone on its edge, an end leans only gently to the middle, so its dates mostly decide.
            pull = 0.3 if n > 1 else 0.08
            for k, (w, side) in enumerate(members):
                target = LO + (k + 0.5) * (HI - LO) / n
                if side == "from":
                    w["T1"] += pull * (clamp(when_at(w["fs"], target), w["f"]) - w["T1"])
                else:
                    w["T2"] += pull * (clamp(when_at(w["ts"], target), w["t"]) - w["T2"])
        for w in work:
            # A gentle pull together: an influence acts about when both phases are under way.
            gap = w["T2"] - w["T1"]
            if gap > 0 and w["meet"]:
                w["T1"] = clamp(w["T1"] + 0.02 * gap, w["f"])
                w["T2"] = clamp(w["T2"] - 0.02 * gap, w["t"])
            elif gap < 0:
                # The effect before its cause: meet at the middle, as far as each phase allows.
                meet = (w["T1"] + w["T2"]) / 2
                w["T1"] = clamp(meet, w["f"])
                w["T2"] = clamp(max(meet, w["T1"]), w["t"])

    # A phase edge whose ends are still bunched is placed outright: its first end as early and its
    # last as late as each one's partner allows, the rest evenly between, in date order.
    def bounds(w, side):
        if side == "from":
            return frac(w["fs"], w["f"][0]), frac(w["fs"], max(min(w["f"][1], w["T2"]), w["f"][0]))
        return frac(w["ts"], min(max(w["t"][0], w["T1"]), w["t"][1])), frac(w["ts"], w["t"][1])

    for _ in range(3):
        for members in slots.values():
            fr = [f_of(*mm) for mm in members]
            if len(members) < 2 or max(fr) - min(fr) >= 0.3:
                continue
            members.sort(key=lambda mm: (f_of(*mm), mm[0]["T1"], mm[0]["T2"]))
            n = len(members)
            lo = bounds(*members[0])[0]
            hi = bounds(*members[-1])[1]
            for k, (w, side) in enumerate(members):
                target = lo + k * (hi - lo) / (n - 1)
                b = bounds(w, side)
                target = min(max(target, b[0]), b[1])
                if side == "from":
                    w["T1"] = when_at(w["fs"], target)
                else:
                    w["T2"] = when_at(w["ts"], target)

    for w in work:
        # Last, strictly: the effect never before its cause, within the phases themselves.
        if w["T1"] > w["T2"]:
            w["T1"] = clamp(w["T2"], w["fs"])
            w["T2"] = clamp(max(w["T2"], w["T1"]), w["ts"])

    out = list(lines)
    for w in work:
        x = infl[w["i"]]
        for side in (("to",) if w.get("trigger") else ("from", "to")):
            # Rounded towards each other - the cause down, the effect up - so rounding never puts
            # the effect's date before the cause's.
            f = f_of(w, side) * 100
            f = (math.floor(f + 1e-9) if side == "from" else math.ceil(f - 1e-9)) / 100
            f = min(max(f, 0.0), 1.0)
            n = x[side + "-at@"]
            out[n] = re.sub(r"(-at: ).*$", lambda mm: mm.group(1) + f"{f:.2f}", out[n])
    return "\n".join(out)


if __name__ == "__main__":
    for path in sys.argv[1:]:
        raw = open(path, "rb").read()
        crlf = b"\r\n" in raw
        text = raw.decode("utf-8").replace("\r\n", "\n")
        new = run(text)
        if crlf:
            new = new.replace("\n", "\r\n")
        open(path, "wb").write(new.encode("utf-8"))
