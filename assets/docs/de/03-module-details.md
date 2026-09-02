---
section: Module
---

# Die Modulseite

Jedes Modul hat seine eigene Seite. Klicke auf eine beliebige Listung auf
dem [Markt](/modules), um sie zu öffnen. Sie enthält die vollständige
Aufschlüsselung der Werte, was das Preismodell für den Wert hält, die
Werkzeuge, die du am häufigsten brauchst, und was ähnliche Module auf dem
Markt gemacht haben.

Die Seite hat drei Teile: die Karte links, das Feld daneben und die Tabs
darunter.

## Die Karte

Die Karte ist das Modul selbst: sein Name, das Modul, aus dem es gerollt
wurde, das verwendete Mutaplasmid und jedes mutierte Attribut mit seinem
gerollten Wert, wie weit dieser sich von der Basis entfernt hat, und ein
Balken, der zeigt, wo der Roll im möglichen Bereich gelandet ist.

Steht es in einem Vertrag, zeigt die Karte den Vertragspreis neben dem
geschätzten Wert. Ein `+N`-Abzeichen bedeutet, dass der Vertrag auch andere
Module enthält.

Attributzeilen sind gefärbt. Grün oder Rot sagt, dass der Roll besser oder
schlechter als der Basiswert ausgefallen ist. Gold, Diamant und Braun
markieren Rolls, die die Extreme des Typs erreicht haben, was
[Den Markt durchstöbern](/documentation/browsing-the-market) erklärt.
Schalte die Attributwertung in den Anzeigeeinstellungen ein und jede Zeile
bekommt außerdem eine vorzeichenbehaftete Wertung dafür, wie weit im Bereich
sie gelandet ist.

## Was es wert ist

Das Feld neben der Karte zeigt, wer das Modul gerollt hat und was das
Preismodell vorhersagt.

| Feld | Bedeutung |
| --- | --- |
| Geschätzter Wert | Der vorhergesagte Preis in ISK. Klicken zum Kopieren. |
| ±% | Der durchschnittliche Fehler des Modells für diesen Modultyp. |
| Ausgewertet | Wann die Schätzung dieses Moduls zuletzt berechnet wurde. |
| Konfidenz | Sehr niedrig bis sehr hoch, aus dem R²-Wert des Modells. |
| Bias-Wert | Wie gleichmäßig die Trainingsdaten die Ausgangstypen abdecken. |
| Durchschn. Fehler (MAE) | Mittlerer absoluter Fehler. Niedriger ist besser. |
| Zuletzt trainiert | Wann das Modell dieses Typs zuletzt neu gebaut wurde. |
| Trainingsdaten | Erfasste Verkäufe pro Ausgangstyp. Weniger als zehn wird rot markiert. |

Ein Typ braucht 50 erfasste Verkäufe, bevor sein Modell trainiert werden
kann. Darunter gibt es keine Vorhersage, und das Feld zeigt stattdessen, wie
viele noch fehlen.

Diese Modelle können deutlich danebenliegen. Nimm die Zahl als Ausgangspunkt
und prüfe, was ähnliche Module tatsächlich kosten. Die
[Bewertungsanleitung](/documentation/appraisal) erklärt, wo sie versagen.

## Die Werkzeugleiste

| Button | Was er tut |
| --- | --- |
| Typ suchen | Der Markt, gefiltert auf den Typ dieses Moduls. |
| Ähnliche suchen | Wähle die abzugleichenden Attribute und eine Toleranz, dann finde Module zum Verkauf mit ähnlichen Rolls. |
| Günstigste suchen | Dasselbe, günstigste zuerst sortiert. |
| Historische suchen | Dasselbe, gegen vergangene Verkäufe statt gegen aktive Listungen. Premium. |
| Pyfa | Kopiert die Werte in einem Format, das Pyfa versteht. |
| Item-Link kopieren | Kopiert einen Link, den du in den Ingame-Notizblock einfügen kannst. |
| Vertragslink kopieren | Kopiert den Vertragslink. Deaktiviert, wenn es keinen Vertrag gibt. |
| Vertrag im Spiel öffnen | Öffnet den Vertrag in deinem EVE-Client. Deaktiviert, wenn es keinen Vertrag gibt. |
| Modul teilen | Teilt oder kopiert einen Link zu dieser Seite. |
| Mehr | Das Share-Bild des Moduls kopieren oder herunterladen. |

Die drei Suchmenüs funktionieren gleich. Hake die Attribute ab, die dir
wichtig sind, lege fest, wie viel Varianz erlaubt ist, und suche. "Alle
auswählen" und "Alle abwählen" schalten alles auf einmal um.

## Kartenmenü

Rechtsklick auf die Karte oder ihr `⋮`-Button öffnet den Rest. Angemeldet
kommt dazu:

**Sammlungen** legt das Modul in eine deiner
[Sammlungen](/collections) oder eine neue und lässt dich dort eine Notiz
daran hinterlassen.

**Werkbank** fügt es deiner Vergleichs-[Werkbank](/documentation/workbench-and-tools)
hinzu oder entfernt es daraus.

Die Kopier- und Exportaktionen aus der Werkzeugleiste sind ebenfalls dort.

**Notiz hinzufügen** hängt eine private Notiz an das Modul. Nur du siehst
sie, und sie erscheint unter der Karte, wo immer das Modul auftaucht.

**Verkaufspreis festlegen** gibt es bei Modulen, die du zum Verkauf gelistet
hast. Beides funktioniert stapelweise: Das Menü schaltet die Bearbeitung
ein, und eine Leiste am unteren Seitenrand speichert.
[Notizen und Verkaufspreise](/documentation/workbench-and-tools) hat die
Details.

## Tabs

**Ausgangstypen** vergleicht jedes Modul, auf das das Mutaplasmid angewendet
werden kann, von T1 bis Officer. Für jedes mutierte Attribut zeigt es den
Basiswert dieses Ausgangsmoduls und wie dieser Roll dagegen abschneidet, plus
was jedes Ausgangsmodul gerade kostet. Das ist der schnellste Weg
herauszufinden, ob dein Roll den Kauf der Fraktionsversion tatsächlich
schlägt.

**Vertragshistorie** ist jeder Vertrag, den MutaMarket für genau dieses
Modul gesehen hat: wer ihn ausgestellt hat, wann, ob er andere Items
enthielt, was mit ihm passiert ist, und der Preis. Der aktive steht als
ausstehend ganz oben.

**Ähnliche Verkäufe** braucht [Premium](/documentation/premium). Es zeigt
Module mit Rolls wie diesem, die tatsächlich verkauft wurden, mit dem
durchschnittlichen, niedrigsten und höchsten Preis. Ohne Premium bekommst du
eine unscharfe Vorschau.

## Es kaufen

Wenn jemand das Modul direkt auf MutaMarket gelistet hat, ist der Preis auf
der Karte anklickbar. Er zeigt den Verkaufspreis oder "Angebot machen", wenn
keiner gesetzt wurde. Das öffnet einen Dialog, in dem du dem Verkäufer
schreibst. Hast du bereits ein offenes Angebot dafür, steht auf der Karte
stattdessen "Zum Angebot". [Angebote](/documentation/offers) beschreibt den
ganzen Ablauf.

Module in einem Ingame-Vertrag werden über den Vertrag gekauft. Nutze
"Vertrag im Spiel öffnen" oder kopiere den Link.
