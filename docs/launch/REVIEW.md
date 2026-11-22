# Clipglass: privates Launch-Paket

**Status: Entwurf. Repo und Post bleiben bis zur ausdrücklichen Freigabe unveröffentlicht.**

## Zur Prüfung

- [README](../../README.md)
- [Installationsanleitung](../INSTALL.md)
- [Testumfang und Messwerte](../TESTING.md)
- [Demo](../media/clipglass-demo.mp4) und Screenshots unter `docs/media/`
- [Demo-Inhalt und Herkunft der Testdaten](DEMO.md)
- [Kurzer Post-Entwurf](POST-DRAFT.md)
- [MIT-Lizenzvorschlag](LICENSE-MIT.proposed.txt)

Die Demo verwendet ausschließlich generierte Texte, Links und Bilder in einer
isolierten macOS-VM. Persönliche History, Raycast-Daten, echte Screenshots und
Zugangsdaten gehören nicht in das Launch-Paket.

## Vor der Veröffentlichung

- Repo-Name und öffentliche URL bestätigen. Aktuell: privates `moritzWa/clip`.
- README, Installationsanleitung, Demo und Post freigeben.
- MIT-Lizenz bestätigen oder andere Lizenz wählen. Der Vorschlag ist noch nicht aktiviert.
- Festlegen, ob zunächst nur der Quellcode veröffentlicht wird. Es wurde kein
  Release-Build, GitHub Release, signierter Download oder Homebrew-Paket erstellt.
- Die Entwicklungsgrenzen im README beibehalten. Besonders wichtig sind das
  mögliche globale Shortcut-Konflikte, unverschlüsselte History und unvollständige
  Passwortmanager-Ausnahmen.
- Nach Freigabe die Lizenz an die endgültige Stelle übernehmen, Links anpassen
  und einen letzten Audit des freizugebenden Standes durchführen.
- Erst danach die ausdrücklich genehmigten GitHub-/Post-Aktionen ausführen.

Es wurden weder Sichtbarkeit noch Repo-Name geändert. Es wurde kein Post
veröffentlicht oder eingeplant.

Die Demo ist 36 Sekunden lang und hat keine Audiospur. Sie zeigt OCR-Suche,
Bild-Zoom, Code-Vorschau und Mehrfach-Paste in der echten App. Alle 20 Unit-Tests
bestanden; ein expliziter Benchmark blieb wie vorgesehen übersprungen. Die
Demo-Prozesse wurden beendet, die VM gestoppt und ihre Lease freigegeben.

## Audit

`scripts/audit-launch.py` prüft alle erreichbaren Git-Blobs auf typische
Secret-Muster, persönliche Home-Pfade, E-Mail-Adressen und verdächtige
Datenbank-/Schlüssel-/Log-Dateien. Treffer werden nur als Regel und Dateiname
ausgegeben, nicht mit dem gefundenen Inhalt. Das ist eine heuristische Prüfung,
kein Beweis, dass sämtliche denkbaren Geheimnisse ausgeschlossen sind.

Der erste Audit des bisherigen Standes `242d21e` umfasste neun Commits und 58
verschiedene Blobs. Er ergab keine Treffer und keine solchen Datendateien.
Private Clipboard-Daten liegen außerhalb dieses Repos. Alte Commits enthalten
Entwicklungsnotizen und persönliche Shortcut-Konfigurationen, aber im geprüften
Stand keine importierte History oder echten Screenshot-Dateien.
