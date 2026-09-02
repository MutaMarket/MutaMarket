---
section: Allgemein
---

# Über

Ich bin Nicolas Kion, und ich habe MutaMarket gebaut.

Es fing an, weil ich jeden Tag dasselbe Problem hatte: Abyssal-Module
verstreut über Charaktere und Container, keine Ahnung, was eines davon wert
war, und kein guter Weg, den bestimmten Roll zu finden, den ich wollte. Die
Ingame-Werkzeuge helfen bei nichts davon.

MutaMarket ist also das, was ich mir gewünscht habe. Du importierst deine
Module aus EVE, es sagt dir, was sie wert sind, basierend darauf, wofür
ähnliche Rolls tatsächlich verkauft wurden, und es gibt dir einen Ort, um
sie zu verkaufen, und einen, um den Überblick über deinen Bestand zu
behalten.

## Wie es gebaut ist

Das Backend ist Rust, mit Axum und Postgres. Das Frontend ist SvelteKit mit
Tailwind. Markt- und Asset-Daten kommen aus EVEs ESI-API.

Die Preisschätzungen stammen aus einem Random Forest, der pro Modultyp auf
echten erfassten Verkäufen trainiert wird. [Bewertung](/documentation/appraisal)
erklärt, wie das funktioniert und wo es versagt.

## Kontakt

Der Abyssal Trading Discord ist der Ort, an dem die Handels-Community ist,
und ich bin dort. Der MutaMarket-Entwicklungs-Discord ist der Ort für Bugs
und Feature-Wünsche. Beide sind im Footer verlinkt.

Du kannst auch Nicolas Kion im Spiel eine Mail schicken oder eine E-Mail an
[nicolaskion07@gmail.com](mailto:nicolaskion07@gmail.com).
