import QtQuick
import qs.Commons

// Properties: label (already translated). No signals.
SectionTitle {
  property string label: ""
  text: label
  topPadding: Style.space(8)
  bottomPadding: Style.space(4)
  fontSize: Style.font.caption
}
