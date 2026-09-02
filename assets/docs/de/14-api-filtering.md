---
section: API
---

# API: Filtern und Sortieren

`GET /api/modules/{query}` nimmt seine Filter und die Sortierung als
URL-Segmente statt als Query-Parameter entgegen. Diese Seite ist das
Vokabular; die Formen von Anfrage und Antwort stehen in der
[Endpunkt-Referenz](/documentation/api).

## Optionen

Das Pfadsegment `query` **muss** eine Typ-Option enthalten
(`type/{id-or-slug}`, z. B. `type/49738` oder
`type/abyssal-ballistic-control-system`) und akzeptiert dieselben
Filteroptionen wie der Modul-Browser, als URL-Segmente in beliebiger
Reihenfolge verkettet.

| Option | Format | Wirkung |
|---|---|---|
| sort | `sort/{field}/{direction}` | Sortieren nach `price` (Vertragspreis), `value` (geschätzter Wert), `fraction` (durchschnittliche Roll-Qualität), `contract-date` (wann der aktuelle Vertrag ausgestellt wurde), `date-added` (wann der aktuelle Vertrag des Moduls zu MutaMarket hinzugefügt wurde) oder einem Dogma-Attribut per Id oder Name (`sort/50/desc`, `sort/cpu/asc`). Die Richtung ist `asc` oder `desc`. Sortieren nach einem Attribut liefert nur Module, die es haben. |
| attributes | `attributes/{attribute}/{value}` (Paare, wiederholbar) | Nach gerollten Werten filtern, z. B. `attributes/cpu/20-30/damageMultiplier/2.1`. Ein `min-max`-Bereich begrenzt den Wert; eine einzelne Zahl ist ein Minimum, wo hoch gut ist, sonst ein Maximum. |
| meta-group | `meta-group/{group}` | Eines von `t1`, `t2`, `storyline`, `faction`, `officer`, `deadspace`: nur Module, die aus einem Ausgangsmodul dieser Meta-Gruppe mutiert wurden. |
| meta-level | `meta-level/{n}` | Nur Module, die aus einem Ausgangsmodul dieses Meta-Levels mutiert wurden. |
| contract-price | `contract-price/{max}` oder `{min}-{max}` | Den Vertragspreis in ISK begrenzen. |
| estimated-value | `estimated-value/{min}` oder `{min}-{max}` | Den geschätzten Wert in ISK begrenzen. |
| goldbar | Flag | Mindestens ein Attribut hat den bestmöglichen Wert des Typs gerollt, auf einem Unstable-Mutaplasmid. |
| brownbar | Flag | Mindestens ein Attribut hat den schlechtestmöglichen Wert des Typs gerollt, auf einem qualifizierenden Mutaplasmid. |
| diamondbar | Flag | Wie goldbar, aber auf einem Glorified-Mutaplasmid gerollt. |
| item-exchange / auction | Flag | Nur Verträge dieses Typs. |
| no-multi-item-contracts | Flag | Nur Verträge, die genau ein Abyssal-Modul und sonst nichts enthalten. |
| contracts-only | Flag | Module ausschließen, die nur als MutaMarket-Verkaufslistung gelistet sind. |
| without-other-items | Flag | Nur Verträge, die keine fremden Items enthalten. |

Verkettet:

```
GET /api/modules/type/abyssal-ballistic-control-system/sort/price/asc/goldbar/contract-price/0-500000000
```

## Nach neuen Listungen abfragen

`sort/date-added/desc` sortiert danach, wann der aktuelle Vertrag eines
Moduls zu MutaMarket hinzugefügt wurde. Diese Reihenfolge ist append-only,
also zeigt das Abfragen der ersten Seite neu gelistete Module, ohne jede
Seite durchzugehen.

## Ein Modul identifizieren

Ein Pfadsegment, das auf Ziffern endet, ist eine Modulsuche per EVE-Item-Id
oder MutaMarket-Slug; alles andere ist die typbezogene Liste. Beide liegen
auf demselben Pfad.

```
GET /api/modules/1052842251186
GET /api/modules/abyssal-ballistic-control-system-1052842251186
```
