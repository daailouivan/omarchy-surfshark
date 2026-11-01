import QtQuick
import QtQuick.Shapes
import qs.Commons

// Precision vector Cyber Shield icon for Surfshark VPN.
// Built on a normalized 24x24 grid with mathematical bezier continuity,
// crisp sub-pixel antialiasing, and responsive active/disconnected states.
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

  Item {
    id: canvas
    width: 24
    height: 24
    anchors.centerIn: parent
    scale: root.iconSize / 24
    transformOrigin: Item.Center

    Shape {
      anchors.fill: parent
      antialiasing: true
      smooth: true
      preferredRendererType: Shape.CurveRenderer

      // Outer Shield contour (smooth mathematical curve)
      ShapePath {
        strokeColor: root.color
        strokeWidth: 1.8
        fillColor: root.active ? Qt.rgba(root.color.r, root.color.g, root.color.b, 0.16) : "transparent"
        capStyle: ShapePath.RoundCap
        joinStyle: ShapePath.RoundJoin

        PathSvg {
          path: "M 12,2.5 "
              + "C 16.2,2.5 19.2,3.6 19.8,4.5 "
              + "C 20.1,5.2 20.2,8.2 20.2,11.5 "
              + "C 20.2,16.2 16.8,19.8 12,21.5 "
              + "C 7.2,19.8 3.8,16.2 3.8,11.5 "
              + "C 3.8,8.2 3.9,5.2 4.2,4.5 "
              + "C 4.8,3.6 7.8,2.5 12,2.5 Z"
        }
      }

      // Security Core / Lock Shackle
      ShapePath {
        strokeColor: root.color
        strokeWidth: 1.8
        fillColor: "transparent"
        capStyle: ShapePath.RoundCap
        joinStyle: ShapePath.RoundJoin

        PathSvg {
          path: root.active
            ? "M 9.5,11 V 8.8 C 9.5,7.3 10.6,6.2 12,6.2 C 13.4,6.2 14.5,7.3 14.5,8.8 V 11"
            : "M 9.5,11 V 8.8 C 9.5,7.3 10.6,6.2 12,6.2 C 13.4,6.2 14.5,7.3 14.5,8.8 V 7.5"
        }
      }

      // Lock Vault Body
      ShapePath {
        strokeColor: root.color
        strokeWidth: 1.8
        fillColor: root.active ? root.color : "transparent"
        capStyle: ShapePath.RoundCap
        joinStyle: ShapePath.RoundJoin

        PathSvg {
          path: "M 8,11 H 16 C 16.6,11 17,11.4 17,12 V 15.8 C 17,16.4 16.6,16.8 16,16.8 H 8 C 7.4,16.8 7,16.4 7,15.8 V 12 C 7,11.4 7.4,11 8,11 Z"
        }
      }

      // Keyhole node
      ShapePath {
        strokeColor: root.active ? "#000000" : root.color
        strokeWidth: 1.5
        fillColor: "transparent"
        capStyle: ShapePath.RoundCap
        joinStyle: ShapePath.RoundJoin

        PathSvg {
          path: "M 12,13 V 14.8"
        }
      }

      // Precision diagonal disconnect slash
      ShapePath {
        strokeColor: root.color
        strokeWidth: 1.8
        fillColor: "transparent"
        capStyle: ShapePath.RoundCap
        joinStyle: ShapePath.RoundJoin

        PathSvg {
          path: (root.crossed && !root.active)
            ? "M 3.5,3.5 L 20.5,20.5"
            : ""
        }
      }
    }
  }
}

