import type { DemoObject } from "./mock-objects";
import { ObjectArt } from "./ObjectArt";

export function ManufactureEvidenceArt({
  object,
  view,
}: {
  object: DemoObject;
  view: number;
}) {
  if (object.objectType !== "模型") {
    return (
      <div className={`op-manufacture-evidence-view is-view-${view}`}>
        <ObjectArt kind={object.kind} annotations={false} />
      </div>
    );
  }
  const side = view === 1;
  const back = view === 2;
  return (
    <svg
      viewBox="0 0 240 280"
      role="img"
      aria-label={`${object.name}${side ? "侧面" : back ? "背面" : "正面"}演示示意图，非实际生成产物`}
    >
      <rect width="240" height="280" fill="#252a32" />
      <path
        d="M0 70H240M0 140H240M0 210H240M60 0V280M120 0V280M180 0V280"
        stroke="#363d48"
        strokeWidth="0.6"
      />
      <ellipse cx="120" cy="260" rx="52" ry="8" fill="#191e27" />
      {side ? (
        <g>
          <path d="M107 176L108 238H119L125 176" fill="#eacbc3" />
          <path d="M121 176L124 242H136L132 176" fill="#f4d7ce" />
          <path d="M106 236H120L127 253H104Z" fill="#343b55" />
          <path d="M123 239H137L146 256H121Z" fill="#343b55" />
          <path d="M108 131L98 184Q120 193 143 184L132 131Z" fill="#3c496b" />
          <path d="M108 91Q120 84 132 99L140 144L103 144Z" fill="#c6d4e2" />
          <path
            d="M116 98L124 147L128 167L137 165L133 141L128 98Z"
            fill="#f4d7ce"
          />
          <path d="M111 98L129 96L132 122L115 125Z" fill="#d9e1e9" />
          <path d="M115 74H128V97H115Z" fill="#eacbc3" />
          <path
            d="M104 44Q131 30 140 51L142 64L150 71L141 74L139 85L113 86L104 69Z"
            fill="#f4d7ce"
          />
          <path
            d="M98 54Q95 28 123 27Q143 28 145 54L126 45L118 72L117 107L94 99Z"
            fill="#6ab8ba"
          />
          <path d="M133 61L141 62" stroke="#344353" strokeWidth="3" />
        </g>
      ) : (
        <g>
          <path d="M97 172L100 243H112L118 172" fill="#eacbc3" />
          <path d="M122 172L129 243H141L143 172" fill="#f4d7ce" />
          <path d="M99 238H113L114 256H91Z" fill="#343b55" />
          <path d="M129 238H142L150 256H127Z" fill="#343b55" />
          <path d="M95 132L82 184Q120 197 158 184L145 132Z" fill="#3c496b" />
          <path
            d="M99 144L96 184M112 147L111 187M130 147L132 187M141 144L147 184"
            stroke="#66708a"
          />
          <path
            d="M98 91L85 120L97 133L99 144H141L143 133L155 120L142 91Z"
            fill="#c6d4e2"
          />
          <path d="M86 116L77 160L80 179L88 178L89 159L99 122" fill="#f4d7ce" />
          <path
            d="M141 122L151 159L152 178L160 179L163 160L154 116"
            fill="#eacbc3"
          />
          <path
            d="M99 91L82 115L97 124L108 99M141 91L158 115L143 124L132 99"
            fill="#d9e1e9"
          />
          <path d="M111 75H129V96H111Z" fill="#eacbc3" />
          <path
            d="M94 45Q120 26 146 45L145 68Q139 85 120 88Q101 85 95 68Z"
            fill="#f4d7ce"
          />
          {back ? (
            <>
              <path
                d="M93 44Q95 24 120 24Q145 24 147 44L151 105Q120 119 89 105Z"
                fill="#6ab8ba"
              />
              <path
                d="M104 41L101 103M136 41L139 103"
                stroke="#4b9299"
                fill="none"
              />
              <path d="M104 113L120 119L136 113" stroke="#788ca3" fill="none" />
            </>
          ) : (
            <>
              <path
                d="M93 47Q91 24 120 24Q148 24 147 48L151 102L137 105L138 51L128 59L119 42L109 58L101 51L103 105L89 102Z"
                fill="#6ab8ba"
              />
              <path
                d="M104 63H113M127 63H136"
                stroke="#344353"
                strokeWidth="3"
              />
              <path d="M116 76Q120 79 124 76" stroke="#bf8e8e" fill="none" />
              <path d="M102 94L120 109L138 94L130 115H110Z" fill="#536983" />
              <path
                d="M108 106L120 111L132 106V120L120 115L108 120Z"
                fill="#be8495"
              />
            </>
          )}
        </g>
      )}
    </svg>
  );
}
