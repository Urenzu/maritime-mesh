interface Props {
  vesselCount: number
  darkCount:   number
  lastUpdate:  string
}

export default function Hud({ vesselCount, darkCount, lastUpdate }: Props) {
  return (
    <div id="hud">
      <div className="hud-product">MM·MESH</div>

      <div className="hud-row">
        <span className="hud-label">Vessels</span>
        <span className="hud-value">{vesselCount.toLocaleString()}</span>
      </div>
      <div className="hud-row">
        <span className="hud-label">Dark</span>
        <span className={`hud-value${darkCount > 0 ? ' active' : ''}`} id="hud-dark">
          {darkCount.toLocaleString()}
        </span>
      </div>

      <div className="hud-sep" />

      <div className="hud-footer">
        <span className="live-dot" />
        <span>{lastUpdate}</span>
      </div>
    </div>
  )
}
