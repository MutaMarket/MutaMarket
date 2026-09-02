---
section: Module
---

# Den Markt durchstöbern

Die [Modulseite](/modules) zeigt, was gerade zum Verkauf steht. Das sind
öffentliche Ingame-Verträge, sowohl Gegenstandsaustausch als auch Auktionen,
plus Module, die Leute direkt auf MutaMarket gelistet haben. Dreißig pro
Seite.

Jede Karte zeigt das Ausgangsmodul und das verwendete Mutaplasmid, jedes
mutierte Attribut mit seinem Wert und wie weit es sich von der Basis entfernt
hat, einen Balken pro Attribut, der zeigt, wo der Roll gelandet ist, und den
Preis. Vertragslistungen zeigen den Vertragspreis neben einem Symbol für
Gegenstandsaustausch oder Auktion, zusammen mit dem geschätzten Wert. Wenn
ein Vertrag mehr als ein Modul enthält, trägt die Karte ein `+N`-Abzeichen
für den Rest.

Klicke auf eine Karte, um ihre [Detailseite](/documentation/module-details)
zu öffnen. Rechtsklick darauf oder der `⋮`-Button öffnet die Schnellaktionen:
ähnliche, günstigste oder vergangene Verkäufe suchen, teilen, den Vertrag
öffnen oder kopieren, Pyfa-Werte oder einen Item-Link kopieren. Angemeldet
bekommst du außerdem Sammlungen, die Werkbank, Notizen und, bei deinen
eigenen Listungen, den Verkaufspreis.

## Filter

Filter liegen in der URL, also lässt sich jede Ansicht, die du baust, als
Lesezeichen speichern, teilen oder später wieder öffnen.
`/modules/type/500mn-abyssal-microwarpdrive/sort/price/asc` ist jeder 500MN
Abyssal-MWD, günstigste zuerst.

| Filter | In der Oberfläche | In der URL |
| --- | --- | --- |
| Typ | Kategorieauswahl | `type/<type-name>` |
| Meta-Gruppe | Meta-Gruppe: Alle, T1, T2, Storyline, Fraktion, Officer, Deadspace | `meta-group/<group>` |
| Meta-Level | Meta-Level | `meta-level/<n>` |
| Attribute | Bereichsregler, sobald ein Typ gewählt ist | `attributes/<name>/<min>-<max>/...` |
| Sortierung | Preis, geschätzter Wert oder ein beliebiges Attribut, in beide Richtungen | `sort/<field>/<asc\|desc>` |
| Preis | Preisregler, 1 Million bis 100 Milliarden ISK | `contract-price/<min>-<max>` |
| Geschätzter Wert | Regler für den geschätzten Wert | `estimated-value/<min>-<max>` |
| Vertragstyp | Alle, Gegenstandsaustausch oder Auktion | `item-exchange` oder `auction` |
| Nur Verträge | Blendet direkte Listungen aus | `contracts-only` |
| Verträge mit mehreren Items | Aus bedeutet nur Verträge mit einem einzelnen Item | `no-multi-item-contracts` |
| Eigene Module | Schließt deine eigenen importierten Assets ein. Braucht einen Account | `with-personal-modules` |
| Jita | Nur Module, die in Jita 4-4 liegen | `in-jita` |
| Gold-, Braun-, Diamantbalken | Unter Sonstiges | `goldbar`, `brownbar`, `diamondbar` |

Im Filterbereich gibt es außerdem einen Button "Pyfa-Modul importieren", der
ein Modul aus Pyfa übernimmt und nach welchen mit ähnlichen Werten sucht.

## Gold-, Braun- und Diamantbalken

Diese markieren Rolls, die ein Extrem dessen erreicht haben, was der
Abyssal-Typ erreichen kann, über jedes Mutaplasmid hinweg, das ihn erzeugt.

Nur die stärkste Stufe für ein Modul bekommt einen. Decayed und Gravid
unterliegen Unstable auf demselben Modul, also bekommen sie nie einen Balken,
und Glorified Decayed oder Glorified Gravid ebenso wenig. Unstable bekommt
einen, genauso Exigent und Radical, über denen es keine stärkere Stufe gibt.

Ein **Goldbalken** ist ein bestmöglicher Roll. Ein **Diamantbalken** ist
derselbe Roll auf einem Glorified-Mutaplasmid. Ein **Braunbalken** ist ein
schlechtestmöglicher Roll.

Ein Attribut, das nicht variieren kann, bekommt gar keinen Balken.

Auf einer Karte sind diese gold, diamantblau oder braun gefärbt, sowohl beim
Wert als auch beim Balken. Die Filter unter Sonstiges grenzen die Liste auf
Module ein, die den gewählten Balken auf mindestens einem Attribut tragen.

## Die Anzeige ändern

Die Optionen über der Liste bleiben zwischen Besuchen erhalten.

**Anzeige** wechselt zwischen Raster, Liste und Tabelle.

**Attributbalken** ändert, was der Balken misst: Standard zeichnet, wo der
Roll innerhalb des Bereichs deines Mutaplasmids gelandet ist, Typ normalisiert
über den gesamten Abyssal-Typ, Absolut nutzt den rohen Wert, und Keine
blendet die Balken aus.

**Attributwertung anzeigen** setzt eine Roll-Qualitätswertung auf jedes
Attribut.

## Marktstatistiken

Das Statistikfeld im Filterbereich zeigt Live-Summen: wie viele Module in der
Datenbank sind, wie viele jeden Balken tragen, wie viele Verträge aktiv sind,
aufgeteilt nach Gegenstandsaustausch und Auktion, und wie viele Module in der
letzten Stunde, am letzten Tag und in der letzten Woche aufgetaucht sind.

## Alle Module

[Alle Module](/all-modules) umfasst alles, was MutaMarket kennt, nicht nur,
was zum Verkauf steht. Verträge spielen keine Rolle, deshalb sind die Filter
enger: Typ- und Meta-Filter, die Balkenschalter, geschätzter Wert und
Attributbereiche. Es ist die Seite für die Frage, welche Rolls es überhaupt
gibt.

## Charaktere

[Charaktere](/characters) listet jeden, der direkt auf MutaMarket verkauft,
Premium-Verkäufer zuerst. Öffne einen, um seine öffentlichen Module zu sehen,
filterbar wie jede andere Liste, seine Beschreibung und alle Discord-,
Twitch- oder Patreon-Accounts, die er anzeigen möchte.

Wechsle auf den Filter "erstellt" und die Seite zeigt, was dieser Charakter
gerollt hat, statt dessen, was er verkauft.

Auf deiner eigenen Seite kannst du deine Beschreibung bearbeiten und
auswählen, welche deiner Asset-Standorte öffentlich sind.

## Statistiken

[Statistiken](/statistics) ordnet Charaktere danach, wie viele
Abyssal-Module sie gerollt haben. Du kannst die Rangliste auf einen Modultyp
eingrenzen oder einen Charakter nach Namen suchen. Ein Klick auf die
Modulanzahl von jemandem öffnet seine Seite, gefiltert auf das, was er
erstellt hat.
