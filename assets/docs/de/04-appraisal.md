---
section: Module
---

# Ein Modul bewerten

Es gibt vier Wege, ein Modul auf MutaMarket zu bringen und herauszufinden,
was es wert ist. Keiner der ersten drei braucht einen Account.

**Einen Item-Link einfügen.** Gehe zur [Bewertungsseite](/modules/add), füge
den Link ein, klicke auf Bewerten. Auf dieser Seite kannst du auch einfach
Strg+V (Cmd+V auf einem Mac) drücken, ohne vorher in das Feld zu klicken.

Um den Link im Spiel zu bekommen, ziehe das Modul in ein Chatfenster, sende
die Nachricht, dann Rechtsklick darauf und kopieren.

**Per Mail einschicken.** Sende deine Modul-Links im Spiel an **MutaMate**.
Du bekommst eine Antwortmail mit einem MutaMarket-Link und einer Schätzung
für jedes einzelne. Mehrere Links in einer Mail sind in Ordnung.

**Deine Assets importieren.** Melde dich an und hole deine Module direkt aus
deinem Ingame-Inventar über [Meine Module](/personal/modules).

Egal welchen Weg du nutzt, das Modul bekommt am Ende seine eigene öffentliche
[Seite](/documentation/module-details), und du landest darauf.

## Was den Preis wirklich bestimmt

Die Werte sind nur ein Teil davon.

Das **Modul, aus dem du gerollt hast**, bringt seine eigenen Werte mit.
X-Type-MWDs sind mehr wert, als du vielleicht erwartest, weil sie keine
Kapazitorstrafe haben, und kein Roll ändert das.

**Das Mutaplasmid-Angebot** zählt. Wenn ein Mutaplasmid knapp ist, findet
selbst ein mittelmäßiger Roll irgendwann einen Käufer.

**Wie viele bereits zum Verkauf stehen** zählt meistens am meisten. Ein
überfüllter Markt drückt die Preise, und am stärksten drückt er auf Module,
die niemand besonders will.

Die Schätzung ist also ein Ausgangspunkt. Um etwas richtig zu bepreisen,
nutze "Ähnliche suchen" oder "Günstigste suchen" auf der Modulseite und
schau, was tatsächlich gelistet ist. Mit [Premium](/documentation/premium)
kannst du weitergehen und mit dem vergleichen, was wirklich verkauft wurde,
über [vergangene Verkäufe](/historic-sales) und den Tab "Ähnliche Verkäufe".

## Wie die Schätzung entsteht

Jeder Abyssal-Typ bekommt sein eigenes Modell, einen Random Forest, der auf
echten Verkäufen trainiert wird.

Die Trainingsdaten sind Verkäufe von Verträgen mit einem einzelnen Modul,
die MutaMarket erfasst hat, mit den Preisen der Basismodule als
Referenzpunkte. Die Eingaben sind die mutierten Attributwerte des Moduls,
das Ziel ist der Verkaufspreis. Ein Random Forest baut viele
Entscheidungsbäume auf verschiedenen Ausschnitten dieser Daten und mittelt
sie, was verhindert, dass ein einzelner Baum überanpasst.

Ein Typ braucht 50 erfasste Verkäufe, bevor er ein Modell bekommt. Darunter
zeigen seine Module statt eines Preises, wie viele noch fehlen. Modelle
werden neu trainiert, sobald neue Verkäufe hinzukommen.

Jede Vorhersage kommt mit ihren eigenen Qualitätszahlen auf der Modulseite,
damit du siehst, wie sehr du ihr trauen kannst. [Die
Modulseite](/documentation/module-details) erklärt jede einzelne.

## Wo es schiefgeht

Das Modell kennt nur, was bereits verkauft wurde, also hinkt es dem Markt um
die Zeit hinterher, die Verkäufe brauchen, um sich anzusammeln. Das wiegt am
schwersten, wenn sich etwas schnell ändert: ein Patch, der das Meta
verschiebt, eine plötzliche Schwemme, ein Mutaplasmid, das selten geworden
ist.

Es ist außerdem genau dort am schwächsten, wo du es am meisten brauchst.
Seltene Module haben wenige vergleichbare Verkäufe. Ungewöhnliche
Wertekombinationen haben vielleicht fast keine. Ein perfekter Roll ist per
Definition etwas, das das Modell kaum gesehen hat. Hochwertige Module sind
die, bei denen der Fehler in ISK am größten ist und bei denen du ohnehin
selbst recherchieren solltest.

Ein einzelner seltsamer Verkauf kann die Zahlen eines Typs verzerren, und
MutaMarket sieht nur die Verträge, die es sehen kann, also haben die Daten
Lücken.

Nutze es, um grob zu wissen, in welcher Preisklasse du bist, und prüfe dann
den Markt.
