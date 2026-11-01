import QtQuick
import QtQuick.Shapes
import qs.Commons

// Redesigned Cyber Shield icon for Surfshark VPN.
// Features a dynamic geometric shield with an embedded secure 'S' waveform.
Item {
  id: root

  property real iconSize: Style.font.icon
  property color color: Color.foreground
  property bool active: false
  property bool crossed: false

  width: iconSize
  height: iconSize
  implicitWidth: iconSize
  implicitHeight: iconSize

  Shape {
    anchors.fill: parent
    antialiasing: true
    smooth: true

    transform: Scale {
      xScale: root.iconSize / 100
      yScale: root.iconSize / 100
    }

    // Outer Shield contour
    ShapePath {
      fillColor: root.active ? Qt.rgba(root.color.r, root.color.g, root.color.b, 0.15) : "transparent"
      strokeColor: root.color
      strokeWidth: 6
      capStyle: ShapePath.RoundCap
      joinStyle: ShapePath.RoundJoin

      PathSvg {
        path: "M 50,10 "
            + "C 68,10 82,16 86,26 "
            + "C 86,56 74,80 50,94 "
            + "C 26,80 14,56 14,26 "
            + "C 18,16 32,10 50,10 Z"
      }
    }

    // Inner stylized 'S' cyber wave
    ShapePath {
      fillColor: "transparent"
      strokeColor: root.color
      strokeWidth: root.active ? 7 : 5
      capStyle: ShapePath.RoundCap
      joinStyle: ShapePath.RoundJoin

      PathSvg {
        path: "M 62,32 "
            + "C 48,26 34,34 36,46 "
            + "C 38,56 64,52 64,66 "
            + "C 64,78 48,82 38,76"
      }
    }

    // Active center lock dot / signal node
    ShapePath {
      fillColor: root.active ? root.color : "transparent"
      strokeColor: "transparent"
      strokeWidth: 0

      PathSvg {
        path: root.active
          ? "M 50,48 A 4,4 0 1,0 50,56 A 4,4 0 1,0 50,48 Z"
          : ""
      }
    }
  }

  // Modern sleek status slash when disconnected
  Rectangle {
    visible: root.crossed
    anchors.centerIn: parent
    width: parent.width * 1.15
    height: Math.max(2, parent.height * 0.10)
    radius: height / 2
    color: root.color
    rotation: -45
    opacity: 0.85
  }
}
