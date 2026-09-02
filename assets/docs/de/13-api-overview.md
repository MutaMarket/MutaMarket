---
section: API
---

# Die MutaMarket-API

MutaMarket hat eine öffentliche, überwiegend lesende HTTP-API für
Abyssal-Module: durchstöbern, was zum Verkauf steht, ein einzelnes Modul
mit jedem gerollten Attribut und seinem geschätzten Wert nachschlagen, ein
Modul aus EVE importieren und die Referenzdaten hinter den
Roll-Qualitätsmetriken lesen.

Sie braucht keinen Schlüssel und keinen Account. Alles ist JSON, über HTTPS,
unter `https://mutamarket.com/api`.

## Bevor du anfängst

- `POST /api/modules` ruft EVEs ESI auf und lässt bei jeder Anfrage ein
  Preismodell laufen. Das dauert Sekunden. Rufe es nicht in einer engen
  Schleife auf.
- Sende einen User-Agent, der dich identifiziert, mit einer Kontaktadresse.
- `/api/abyssal-type-statistics` ändert sich ein paar Mal im Jahr. Cache es.

## Konventionen

Einzelne Objekte kommen in einen `data`-Schlüssel gehüllt; die
Referenz-Endpunkte liefern ein nacktes Array.

Fehler sind immer ein JSON-Objekt mit einer `message`, und der HTTP-Status
trägt die Bedeutung:

| Status | Bedeutung |
|---|---|
| 400 | Die Anfrage wurde verstanden, konnte aber nicht ausgeführt werden. |
| 404 | Kein solches Modul, oder die Abfrage nannte keinen gültigen Abyssal-Typ. |
| 422 | Die Anfrage war wohlgeformt, aber ein Wert war nicht akzeptabel. `errors` nennt die Felder. |
| 500 | Unser Fehler. |

```json
{ "message": "Please provide a valid type." }
```

## Welche Endpunkte stabil sind

Nur die in diesem Abschnitt dokumentierten Endpunkte sind öffentlich, und
nur sie tragen ein Kompatibilitätsversprechen: Wir entfernen kein Feld und
ändern seine Bedeutung nicht ohne Ankündigung.

Alles andere unter `/api` bedient mutamarket.com selbst und ändert sich ohne
Vorwarnung. Wenn du etwas brauchst, das nur dort verfügbar ist, frag danach,
statt dich darauf zu verlassen.

## Referenz

Die [Endpunkt-Referenz](/documentation/api) listet jeden Endpunkt mit seinen
Parametern, Antworten und Schemas.
[`/api/openapi.json`](/api/openapi.json) ist dieselbe Beschreibung in
maschinenlesbarer Form.

## Kontakt

Bugs, fehlende Daten oder ein Endpunkt, den du dir wünschst: siehe
[Support](/documentation/support).
