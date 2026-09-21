import { useId } from "react";
import type { ArtKind } from "./mock-objects";
import { CharacterArt } from "./CharacterArt";

function Classroom() {
  return (
    <g stroke="#6f817e" strokeWidth="1.5" strokeLinejoin="round">
      <path d="M0 0h600v420H0z" fill="#e0dfcc" stroke="none" />
      <path d="M0 0l158 78h442V0 M158 78v194L0 420V0" fill="#d0d5c6" />
      <path d="M158 272h442v148H0z" fill="#bba381" />
      <path d="M158 260h442v12H158z" fill="#7f8980" />
      {[40, 112, 211, 337, 484, 648].map((x) => (
        <path key={x} d={`M${x} 420L${158 + x * 0.62} 272`} opacity=".4" />
      ))}
      {[298, 331, 372].map((y) => (
        <path key={y} d={`M0 ${y}h600`} opacity=".4" />
      ))}
      <path
        d="M200 112h235v122H200z"
        fill="#506f67"
        stroke="#a7a189"
        strokeWidth="7"
      />
      <path d="M208 227h219" stroke="#dddac2" strokeWidth="3" />
      <path
        d="M226 138h44m-44 8h75m-76 20h36m-36 8h55"
        stroke="#d1dbca"
        opacity=".5"
      />
      <path
        d="M36 47l94 43v170L36 336z"
        fill="#adcacf"
        stroke="#eff2df"
        strokeWidth="9"
      />
      <path d="M39 194l90-15 M82 70v225" stroke="#f2efdb" strokeWidth="6" />
      <path
        d="M40 248l84-61v69l-84 69"
        fill="#819e88"
        stroke="none"
        opacity=".65"
      />
      <path
        d="M41 327l89-67 348 134-258 26H57z"
        fill="#f2d9a2"
        opacity=".58"
        stroke="none"
      />
      <path
        d="M476 119h76v146h-76z"
        fill="#c4c8b8"
        stroke="#9ba598"
        strokeWidth="5"
      />
      <path d="M487 133h52v63h-52z" fill="#9fb8b8" />
      <circle cx="485" cy="217" r="2" fill="#657470" />
      <circle cx="463" cy="90" r="14" fill="#eeeede" stroke="#9ba497" />
      <path d="M463 80v11l7 4" fill="none" />
      <path d="M180 30h93l39 14h-96z M427 30h96l39 14H463z" fill="#f8f4df" />
      {[0, 1, 2].map((row) =>
        [0, 1].map((column) => {
          const x = 171 + column * 282 - row * 28,
            y = 285 + row * 49;
          return (
            <g
              key={`${row}-${column}`}
              transform={`translate(${x} ${y}) scale(${0.6 + row * 0.18})`}
            >
              <path d="M0 0l87-4 28 16-94 7z" fill="#bd9b6a" stroke="#8b795e" />
              <path
                d="M4 5v47m24-35v48m76-52v42"
                stroke="#687876"
                strokeWidth="4"
              />
              <path
                d="M32 38l49-3 21 14-49 4z M36 13l43-3v24l-41 3z"
                fill="#b09368"
              />
              <path
                d="M39 30v39m56-21v24 M54 52v22"
                stroke="#687876"
                strokeWidth="3"
              />
            </g>
          );
        }),
      )}
    </g>
  );
}

export function ObjectArt({
  kind,
  historical = false,
  annotations = true,
}: {
  kind: ArtKind;
  historical?: boolean;
  annotations?: boolean;
}) {
  const id = useId().replaceAll(":", "");
  return (
    <svg
      className={`op-art op-art-${kind}`}
      viewBox="0 0 600 420"
      role="img"
      aria-label={`${kind === "character" ? "NPR 角色" : kind === "classroom" ? "午后教室" : kind === "performance" ? "教室舞蹈" : kind === "texture" ? "木纹贴图" : kind === "code" ? "控制器逻辑" : "演出界面"}演示插画，非实际生成产物`}
    >
      <defs>
        <pattern
          id={`${id}-grid`}
          width="30"
          height="30"
          patternUnits="userSpaceOnUse"
        >
          <path
            d="M30 0H0v30"
            fill="none"
            stroke="#819499"
            strokeOpacity=".12"
          />
        </pattern>
        <pattern
          id={`${id}-wood`}
          width="100"
          height="140"
          patternUnits="userSpaceOnUse"
        >
          <rect width="100" height="140" fill="#c1a17b" />
          <path d="M0 0v140M99 0v140" stroke="#937652" />
          <path
            d="M11 0q24 40 3 80t8 60 M26 0q-9 34 7 85t-2 55 M52 0q-27 73 8 120l4 20 M81 0q-16 23-5 57t10 83"
            stroke="#ab895f"
            fill="none"
            strokeWidth="2"
          />
          <ellipse
            cx="53"
            cy="69"
            rx="8"
            ry="28"
            fill="none"
            stroke="#ad8b65"
          />
        </pattern>
      </defs>
      {kind === "character" ? (
        <>
          <rect width="600" height="420" fill="#d9e1df" />
          <rect width="600" height="420" fill={`url(#${id}-grid)`} />
          <circle cx="298" cy="206" r="164" fill="#eef0e6" opacity=".7" />
          {annotations && (
            <>
              <path
                d="M102 362h397M300 21v377"
                stroke="#819499"
                opacity=".3"
                strokeDasharray="3 5"
              />
              <text
                x="26"
                y="43"
                fill="#6d858b"
                fontSize="13"
                letterSpacing="3"
              >
                MIO / 01
              </text>
              <text x="26" y="64" fill="#819499" fontSize="9" letterSpacing="2">
                CHARACTER STUDY
              </text>
              <g fill="#788e93">
                <circle cx="491" cy="329" r="7" fill="#618e96" />
                <circle cx="491" cy="352" r="7" fill="#364858" />
                <circle cx="491" cy="375" r="7" fill="#b9765e" />
              </g>
            </>
          )}
          <CharacterArt historical={historical} />
        </>
      ) : kind === "classroom" || kind === "performance" ? (
        <>
          <Classroom />
          {kind === "performance" && (
            <g transform="translate(66 57) scale(.83)">
              <CharacterArt />
            </g>
          )}
        </>
      ) : kind === "texture" ? (
        <>
          <rect width="600" height="420" fill={`url(#${id}-wood)`} />
          <path
            d="M0 140h100m0 140h100M200 90h100m0 140h100m0-150h100m0 230h100"
            stroke="#937652"
          />
        </>
      ) : kind === "code" ? (
        <>
          <rect width="600" height="420" fill="#15232b" />
          <rect width="600" height="420" fill={`url(#${id}-grid)`} />
          <text
            x="43"
            y="65"
            fill="#a4c9c6"
            fontFamily="monospace"
            fontSize="16"
          >
            DanceController.gd
          </text>
          {[
            "extends Node",
            "",
            "@export var tempo = 120",
            "@onready var player = $AnimationPlayer",
            "",
            "func play_dance():",
            '    player.play("after_school")',
            "    beat_sync.start()",
          ].map((line, i) => (
            <text
              key={i}
              x="44"
              y={106 + i * 28}
              fill={i === 5 ? "#c5dccb" : "#8ca6b4"}
              fontFamily="monospace"
              fontSize="14"
            >
              {line}
            </text>
          ))}
          {annotations && (
            <text x="44" y="385" fill="#83969c" fontSize="12">
              代码示意 · 暂无运行截图
            </text>
          )}
        </>
      ) : (
        <>
          <Classroom />
          <rect width="600" height="420" fill="#162632" opacity=".62" />
          <rect x="68" y="110" width="464" height="194" rx="8" fill="#f3f1e6" />
          <text x="99" y="154" fill="#344950" fontSize="13" letterSpacing="3">
            AFTER SCHOOL
          </text>
          <text x="99" y="190" fill="#344950" fontSize="24">
            放课后的舞步
          </text>
          <path d="M104 232h387" stroke="#b6c7c3" strokeWidth="4" />
          <path d="M104 232h148" stroke="#547e84" strokeWidth="4" />
          <circle cx="252" cy="232" r="6" fill="#547e84" />
          <path d="M106 261v17l14-8z" fill="#344950" />
          <text x="139" y="275" fill="#687e80" fontSize="12">
            00:24 / 01:08
          </text>
          <text x="401" y="275" fill="#687e80" fontSize="12">
            SPACE 暂停
          </text>
        </>
      )}
    </svg>
  );
}
