import QtQuick
import qs.Commons as Commons

// Properties: text; tone = neutral, muted, accent, good/success, warning, danger/error.
// Empty text collapses. No signals; callers translate static text, not backend data.
SettingsCaption {
  property string tone: "neutral"
  visible: text !== ""
  color: tone === "danger" || tone === "error" || tone === "warning" ? Commons.Color.urgent
    : tone === "accent" || tone === "good" || tone === "success" ? Commons.Color.accent
    : tone === "muted" ? Commons.Color.muted : Commons.Color.foreground
}
