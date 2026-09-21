export function CharacterArt({ historical = false }: { historical?: boolean }) {
  const hair = historical ? "#739496" : "#618e96";
  return (
    <g
      stroke="#293b49"
      strokeWidth="2.2"
      strokeLinejoin="round"
      strokeLinecap="round"
    >
      <ellipse
        cx="300"
        cy="399"
        rx="87"
        ry="9"
        fill="#2c414d"
        opacity=".13"
        stroke="none"
      />
      <path
        d="M280 293l-3 57 7 37h15l4-88 M312 295l5 53-3 39h16l10-43-5-54"
        fill="#edc6b8"
      />
      <path d="M279 343l4 42h16l-1-38 M317 346l-3 40h16l7-44" fill="#374450" />
      <path
        d="M284 382l-12 12q0 6 27 3l3-14 M315 382l-3 15q21 4 31-1l-14-14"
        fill="#283441"
      />
      <path d="M267 221l-22 76q53 27 104 1l-24-80" fill="#364858" />
      <path
        d="M273 233l-13 64 15 5 8-64 M298 239l-1 68 13-1-2-68 M324 234l16 67 9-3-20-61"
        fill="#526879"
        stroke="none"
      />
      <path
        d="M249 286q48 22 96 1 M247 291q47 23 101 1"
        fill="none"
        stroke="#b0c2c8"
        strokeWidth="1.2"
      />
      <path
        d="M267 153l-19 11-22 62 13 7 29-47 M325 153l23 15 31 54-13 9-36-43"
        fill="#f3efe1"
      />
      <path
        d="M229 221l-12 38-7 18 6 4 9-11 2 11 7-3 2-19 9-28 M367 220l17 29 3 19 7 2 1-12 7 6 4-5-13-19-17-24"
        fill="#edc6b8"
      />
      <path
        d="M265 153l13-8h41l14 12-9 43 7 36q-36 15-65 0l7-35z"
        fill="#f4f0e2"
      />
      <path
        d="M273 170l6 27-6 36 M317 174l-5 28 11 28"
        fill="none"
        stroke="#c9c7bd"
      />
      <path d="M284 137v17l14 18 16-18-4-20" fill="#edc6b8" />
      <path d="M283 142l27-2-1 9-12 7-12-5" fill="#d9a697" stroke="none" />
      <path
        d="M280 148l18 22-20 26-13-39 M316 149l-18 21 21 25 13-37"
        fill="#354b5c"
      />
      <path
        d="M278 154l-7 6 8 25 15-16 M318 154l8 7-8 23-15-14"
        fill="none"
        stroke="#e4e9df"
        strokeWidth="1.3"
      />
      <path d="M298 171l-19 6 6 13 13-11 13 12 9-13z" fill="#b9765e" />
      <path d="M298 177l-6 26 8-5 7 5-6-26" fill="#ca9372" />
      <path
        d="M256 90q-9 38 0 67l-8 8 17-5 11-30h47l10 32 17 3-9-16q7-33-6-58z"
        fill="#395d6c"
      />
      <path
        d="M265 98q-5 31 8 41l24 16 24-14q18-18 13-42l-29-22z"
        fill="#f4d6c3"
      />
      <path
        d="M267 111l8 22 22 17 23-16 11-22-3 28-31 17-27-20z"
        fill="#e7b9aa"
        stroke="none"
      />
      <path
        d="M255 108q-14-41 14-57 27-23 57-3 31 15 16 69l-12-28-14-17-3 28-14-21-9 24-6-28-16 36-1-24z"
        fill={hair}
      />
      <path
        d="M270 58l-10 40 20-35-5 21 16-26-4 21 16-24 18 15-5-18q-27-15-46 6"
        fill="#a6c8c5"
        stroke="none"
      />
      <path
        d="M326 65q17 17 9 35 M262 104l-4 41 M330 118l5 24"
        fill="none"
        stroke="#82aeb2"
        strokeWidth="4"
      />
      <path
        d="M273 112q8-7 17 0 M306 112q10-7 18 0"
        fill="none"
        strokeWidth="2.8"
      />
      <path
        d="M277 112v6q5 9 10 0v-6 M310 112v7q5 8 10-1v-6"
        fill="#5c7e86"
        strokeWidth="1"
      />
      <path d="M280 113v4 M313 113v4" stroke="#fff5dc" strokeWidth="2.5" />
      <path
        d="M272 123l7 1 M318 124l7-2"
        stroke="#d28e82"
        strokeWidth="2"
        opacity=".65"
      />
      <path
        d="M297 119l-2 7 3 1 M292 136q6 3 12-1"
        fill="none"
        stroke="#ae786e"
        strokeWidth="1.2"
      />
      <path d="M324 94l10-6 M325 100l11-6" stroke="#e9d7a6" strokeWidth="3" />
      <path
        d="M256 170l-16 49 7 3 M343 180l23 37-6 5"
        fill="none"
        stroke="#a0afb1"
        strokeWidth="1.5"
      />
      <path
        d="M264 231q33 12 64 0"
        fill="none"
        stroke="#293b49"
        strokeWidth="4"
      />
    </g>
  );
}
