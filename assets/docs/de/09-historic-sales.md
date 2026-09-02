---
section: Handel
---

# Vergangene Verkäufe

[Vergangene Verkäufe](/historic-sales) ist jeder Verkauf eines
Abyssal-Moduls, den MutaMarket erfasst hat: das Modul, wofür es tatsächlich
verkauft wurde, und wann. Es sind die Daten, auf denen die Preismodelle
trainiert werden, und das Beste, wogegen du Preise abgleichen kannst.

Es braucht [Premium](/documentation/premium). Ohne schickt dich die Seite
zur Premium-Seite.

## Woher die Daten kommen

MutaMarket durchsucht öffentliche Verträge im bekannten Raum. Wenn ein
Vertrag mit einem einzelnen Modul verschwindet, fragt die Seite EVEs API,
was damit passiert ist. Wurde er abgeschlossen, wird der Verkauf zu seinem
Endpreis erfasst. Ist er abgelaufen oder wurde gelöscht, wird nichts
erfasst.

Deshalb besteht der Datensatz nur aus Verträgen mit einem einzelnen Modul.
Ein Vertrag mit vier Modulen und einem Schiff darin hat keinen Preis, der zu
einem einzelnen Modul gehört.

## Darin suchen

Dieselben URL-basierten Filter wie auf dem Markt: Typ, Attributbereiche,
Verkaufspreis, sortiert nach Preis oder nach Datum, neueste zuerst.

Normalerweise kommst du von einem Modul aus hierher, statt bei null
anzufangen. "Historische suchen" in der Werkzeugleiste oder im
Rechtsklickmenü jedes Moduls lässt dich wählen, welche Attribute abgeglichen
werden und wie viel Varianz erlaubt ist, und setzt dich bei den
vergleichbaren Verkäufen ab. Der Tab "Ähnliche Verkäufe" auf einer
[Modulseite](/documentation/module-details) tut dasselbe, mit dem
durchschnittlichen, niedrigsten und höchsten Preis bereits ausgerechnet. Das
Trainingsdaten-Feld verlinkt auf die Verkäufe für diesen Typ.

## Warum das von der Schätzung abweicht

Vergangene Preise sind einzelne Verkäufe, also enthalten sie die
Glückstreffer und die Notverkäufe. Die Schätzung ist ein Modell, das über
viele davon mittelt.

Wenn die beiden auseinanderliegen, sind die Verkäufe der bessere Beleg.
Schau dir mehrere aktuelle vergleichbare an und ermittle, wo dein Roll
dazwischen liegt.
