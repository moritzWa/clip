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
print(root)
