#!/usr/bin/env python3
"""Vector icon studies. No remote services or personal data."""
from pathlib import Path

root = Path(__file__).resolve().parent.parent / "docs/icons"
root.mkdir(parents=True, exist_ok=True)
defs = '''
<defs>
  <linearGradient id="blue" x1="0" y1="0" x2=".7" y2="1"><stop stop-color="#78dbff"/><stop offset=".42" stop-color="#248bef"/><stop offset="1" stop-color="#1654c1"/></linearGradient>
  <linearGradient id="dark" x2="0" y2="1"><stop stop-color="#42464e"/><stop offset=".55" stop-color="#25282f"/><stop offset="1" stop-color="#14171e"/></linearGradient>
  <linearGradient id="silver" x1=".2" y1="0" x2=".8" y2="1"><stop stop-color="#fff"/><stop offset=".35" stop-color="#f3f5f7"/><stop offset="1" stop-color="#c2cbd6"/></linearGradient>
  <linearGradient id="glass" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#fff" stop-opacity=".98"/><stop offset=".38" stop-color="#f0fcff" stop-opacity=".91"/><stop offset="1" stop-color="#c5e7fa" stop-opacity=".82"/></linearGradient>
  <linearGradient id="aqua" x2=".7" y2="1"><stop stop-color="#b7fff7"/><stop offset="1" stop-color="#42bcf2"/></linearGradient>
  <linearGradient id="edge" x2=".5" y2="1"><stop stop-color="#fff" stop-opacity=".9"/><stop offset=".45" stop-color="#fff" stop-opacity=".12"/><stop offset="1" stop-color="#fff" stop-opacity=".3"/></linearGradient>
  <linearGradient id="clip" x1="0" y1="0" x2=".3" y2="1"><stop stop-color="#fff"/><stop offset=".55" stop-color="#ebf1f7"/><stop offset="1" stop-color="#b3c5d9"/></linearGradient>
  <filter id="object" x="-40%" y="-40%" width="180%" height="200%"><feDropShadow dx="0" dy="20" stdDeviation="16" flood-color="#031738" flood-opacity=".3"/><feDropShadow dx="0" dy="3" stdDeviation="2" flood-color="#082c59" flood-opacity=".2"/></filter>
  <filter id="tile" x="-30%" y="-30%" width="160%" height="170%"><feDropShadow dx="0" dy="14" stdDeviation="12" flood-opacity=".22"/></filter>
</defs>'''
shape = 'M282 64H742C896 64 960 128 960 282V742C960 896 896 960 742 960H282C128 960 64 896 64 742V282C64 128 128 64 282 64Z'


def icon(name, title, background, content):
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024" role="img" aria-label="{title}">
<title>{title}</title>{defs}
<path d="{shape}" fill="url(#{background})" filter="url(#tile)"/>
<path d="{shape}" transform="translate(4 4) scale(.9921875)" fill="none" stroke="url(#edge)" stroke-width="3"/>
{content}</svg>'''
    (root / f"{name}.svg").write_text(svg)


icon("01-frost", "Frost: a glass clipboard on blue", "blue", '''
<g filter="url(#object)">
  <rect x="296" y="260" width="432" height="536" rx="65" fill="#0c4899" opacity=".18" transform="translate(0 12)"/>
  <rect x="296" y="260" width="432" height="536" rx="65" fill="url(#glass)" stroke="url(#edge)" stroke-width="5"/>
  <rect x="307" y="271" width="410" height="514" rx="54" fill="none" stroke="white" stroke-opacity=".36" stroke-width="2"/>
  <rect x="400" y="213" width="224" height="112" rx="45" fill="url(#clip)" stroke="#fff" stroke-width="4"/>
  <path d="M443 270H581" fill="none" stroke="#819ab7" stroke-opacity=".5" stroke-width="12" stroke-linecap="round"/>
  <path d="M383 431H641M383 525H641M383 619H555" fill="none" stroke="#337bcc" stroke-opacity=".61" stroke-width="24" stroke-linecap="round"/>
</g>''')

icon("02-stack", "Stack: two overlapping glass sheets on graphite", "dark", '''
<g filter="url(#object)">
  <rect x="278" y="237" width="390" height="486" rx="58" fill="url(#aqua)" transform="rotate(-9 473 480)" stroke="url(#edge)" stroke-width="4"/>
  <rect x="367" y="328" width="390" height="486" rx="58" fill="url(#glass)" stroke="url(#edge)" stroke-width="5"/>
  <rect x="378" y="339" width="368" height="464" rx="47" fill="none" stroke="#fff" stroke-opacity=".38" stroke-width="2"/>
  <path d="M443 455H680M443 544H680M443 633H595" fill="none" stroke="#5689b1" stroke-opacity=".56" stroke-width="22" stroke-linecap="round"/>
</g>''')

icon("03-silver", "Silver: a sculpted metal clipboard on graphite", "dark", '''
<g filter="url(#object)">
  <rect x="298" y="258" width="428" height="540" rx="68" fill="url(#silver)" stroke="url(#edge)" stroke-width="5"/>
  <rect x="318" y="278" width="388" height="500" rx="48" fill="none" stroke="#8b9bab" stroke-opacity=".35" stroke-width="3"/>
  <rect x="402" y="213" width="220" height="116" rx="44" fill="url(#clip)" stroke="#fff" stroke-width="4"/>
  <path d="M445 269H579" fill="none" stroke="#748497" stroke-opacity=".56" stroke-width="12" stroke-linecap="round"/>
  <path d="M389 440H635M389 531H635M389 622H553" fill="none" stroke="#798a9d" stroke-opacity=".67" stroke-width="22" stroke-linecap="round"/>
  <circle cx="641" cy="719" r="18" fill="url(#blue)"/>
</g>''')
# Second round: genuinely translucent objects, no document lines, a stronger
# indigo/violet field and larger signature dots. The first round stays intact.
defs = defs.replace("</defs>", '''
  <linearGradient id="indigo" x1=".12" y1="0" x2=".88" y2="1"><stop stop-color="#a0c0ff"/><stop offset=".32" stop-color="#506CF7"/><stop offset=".7" stop-color="#6744e9"/><stop offset="1" stop-color="#25258b"/></linearGradient>
  <radialGradient id="bloom" cx=".18" cy=".12" r=".9"><stop stop-color="#c1f4ff" stop-opacity=".28"/><stop offset=".6" stop-color="#92b8ff" stop-opacity="0"/><stop offset="1" stop-color="#5036ce" stop-opacity="0"/></radialGradient>
  <linearGradient id="clear" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#fff" stop-opacity=".30"/><stop offset=".32" stop-color="#e4efff" stop-opacity=".09"/><stop offset=".67" stop-color="#e0e9ff" stop-opacity=".16"/><stop offset="1" stop-color="#fff" stop-opacity=".33"/></linearGradient>
  <linearGradient id="rear" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#d2faff" stop-opacity=".24"/><stop offset=".7" stop-color="#dddbff" stop-opacity=".06"/><stop offset="1" stop-color="#fff" stop-opacity=".19"/></linearGradient>
  <linearGradient id="rim" x1="0" y1="0" x2=".8" y2="1"><stop stop-color="white" stop-opacity=".96"/><stop offset=".35" stop-color="#e8f4ff" stop-opacity=".34"/><stop offset=".62" stop-color="#c8d6ff" stop-opacity=".13"/><stop offset="1" stop-color="white" stop-opacity=".73"/></linearGradient>
  <linearGradient id="chrome" x2="0" y2="1"><stop stop-color="#fff"/><stop offset=".16" stop-color="#e9edf5"/><stop offset=".42" stop-color="#b7c0d1"/><stop offset=".49" stop-color="#7f8b9f"/><stop offset=".53" stop-color="#dfe7f3"/><stop offset=".78" stop-color="#f5f8fc"/><stop offset="1" stop-color="#9aa7bb"/></linearGradient>
  <linearGradient id="alloy" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#fff"/><stop offset=".28" stop-color="#edf0f6"/><stop offset=".55" stop-color="#ced5e2"/><stop offset=".78" stop-color="#f2f4f8"/><stop offset="1" stop-color="#a9b6cb"/></linearGradient>
  <radialGradient id="orb" cx=".28" cy=".18" r=".9"><stop stop-color="#a3efff"/><stop offset=".34" stop-color="#617fff"/><stop offset=".72" stop-color="#506CF7"/><stop offset="1" stop-color="#303fc3"/></radialGradient>
  <filter id="soft" x="-40%" y="-40%" width="180%" height="190%"><feDropShadow dx="0" dy="16" stdDeviation="14" flood-color="#13134c" flood-opacity=".20"/></filter>
</defs>''')

bloom = f'<path d="{shape}" fill="url(#bloom)"/>'
rear = '''<rect x="261" y="218" width="390" height="500" rx="64" fill="url(#rear)" stroke="url(#rim)" stroke-width="3" transform="rotate(-9 456 468)"/>'''
front = '''<rect x="354" y="324" width="394" height="476" rx="64" fill="url(#clear)" stroke="url(#rim)" stroke-width="5"/>
<rect x="364" y="334" width="374" height="456" rx="54" fill="none" stroke="white" stroke-opacity=".17" stroke-width="2"/>'''
clip = '''<g filter="url(#soft)"><rect x="439" y="290" width="224" height="76" rx="29" fill="url(#chrome)" stroke="#eaf0fc" stroke-width="2"/>
<rect x="448" y="299" width="206" height="58" rx="20" fill="none" stroke="#fff" stroke-opacity=".36" stroke-width="2"/>
<path d="M492 329H610" stroke="#68758a" stroke-opacity=".55" stroke-width="8" stroke-linecap="round"/></g>'''
dot = '''<circle cx="650" cy="699" r="51" fill="url(#orb)" stroke="#a6b8ff" stroke-opacity=".58" stroke-width="2"/>'''

icon("04-dual-glass", "Dual Glass: transparent copies and a metal clip", "indigo",
     bloom + '<g filter="url(#soft)">' + rear + front + '</g>' + clip)
icon("05-glass-dot", "Glass Dot: layered transparent glass and an indigo orb", "indigo",
     bloom + '<g filter="url(#soft)">' + rear + front + '</g>' + clip + dot)
icon("06-pure-copy", "Pure Copy: two clear sheets and one blue dot", "indigo",
     bloom + '<g filter="url(#soft)">' + rear + front + '</g>' + dot)

silver_front = '''<g filter="url(#object)"><rect x="354" y="324" width="394" height="476" rx="64" fill="url(#alloy)" stroke="url(#rim)" stroke-width="4"/>
<rect x="365" y="335" width="372" height="454" rx="53" fill="none" stroke="#fff" stroke-opacity=".4" stroke-width="2"/></g>'''
icon("07-silver-dot", "Silver Dot: abstract metal clipboard with a larger indigo accent", "dark",
     rear + silver_front + clip + dot)
icon("08-indigo-alloy", "Indigo Alloy: sculpted silver over a violet blue gradient", "indigo",
     bloom + rear + silver_front + clip + dot)
icon("09-orbit", "Orbit: a silver plate with an oversized blue glass lens", "dark",
     rear + silver_front + '''<rect x="455" y="294" width="192" height="53" rx="26.5" fill="url(#chrome)" stroke="#eef4ff" stroke-width="2"/>
<g filter="url(#soft)"><circle cx="652" cy="694" r="79" fill="url(#chrome)"/>
<circle cx="652" cy="694" r="73" fill="url(#orb)" stroke="#bccdff" stroke-width="2"/>
<path d="M605 667C616 635 645 625 668 634" fill="none" stroke="#e9f8ff" stroke-opacity=".65" stroke-width="5" stroke-linecap="round"/></g>''')
print(root)
