#!/usr/bin/env -S deno run --allow-all

await main();

async function main() {
  const regions = await getRegions();

  console.dir(regions, { depth: 5 });

  console.log(enumeration(regions, false));
  console.log();

  console.log(parser(regions));
  console.log();

  console.log(details(regions) + ";");
  console.log();
}

async function getRegions() {
  const query = `{\nplatform {\nregions {\nname\ncode\nlatitude\nlongitude\n}\n}\n}`;

  const response = await fetch("https://api.fly.io/graphql", {
    method: "POST",
    headers: {
      Accept: "application/json",
      Authorization: `bearer ${await token()}`,
      "Content-Type": "application/json",
      Origin: "https://api.fly.io",
      "Fly-GraphQL-Client": "playground",
    },
    body: JSON.stringify({ query }),
  });

  const {
    data: {
      platform: { regions },
    },
  } = await response.json();

  return new Map(
    regions.map(({ name, code, latitude, longitude }) => [
      code,
      {
        code,
        name,
        city: {
          name: city(name),
          country: country(name),
          geo: { latitude, longitude },
        },
      },
    ])
  );
}

function enumeration(regionMap, flip) {
  const regions = [...regionMap.values()].sort((a, b) =>
    a.city.name.localeCompare(b.city.name)
  );

  const entry = (region) =>
    [
      `  /// The _${region.name}_ Fly.io region (\`${region.code}\`).`,
      `  #[cfg_attr(feature = "serde", serde(rename = ${repr(region.code)}))]`,
      `  ${key(region)} = ${discriminant(region, flip)},`,
    ].join("\n");

  return `pub enum Region {\n${regions.map(entry).join("\n")}\n}\n`;
}

function discriminant(region, flip) {
  const { code } = region;

  return `0x${flip ? hex(reverse(code)) : `${hex(code)}00`}`;
}

function parser(regionMap) {
  const regions = [...regionMap.values()];

  return [
    `impl FromStr for Region {`,
    `    type Err = RegionError;`,
    ``,
    `    fn from_str(s: &str) -> Result<Self, Self::Err> {`,
    `        match s {`,
    ...regions.map(
      (region) =>
        `            ${repr(region.code)} => Ok(Self::${key(region)}),`
    ),
    `            _ => Err(RegionError::Unrecognized),`,
    `        }`,
    `    }`,
    `}`,
  ].join("\n");
}

function details(regionMap) {
  const regions = [...regionMap.values()];

  return [
    `static ref DETAILS: EnumMap<Region, RegionDetails<'static>> = enum_map! {`,
    ...regions.map(
      (region) =>
        `  Region::${key(region)} => RegionDetails::new(${repr(
          region.code
        )}, ${repr(region.name)}, ${repr(region.city.name)}, ${repr(
          region.city.country
        )}, [${region.city.geo.latitude}, ${region.city.geo.longitude}]),`
    ),
    `}`,
  ].join("\n");
}

async function token() {
  const command = new Deno.Command("fly", { args: ["auth", "token"] });
  const { code, stdout } = await command.output();
  console.assert(code === 0, `fly exited with status ${code}`);

  const output = new TextDecoder().decode(stdout);
  return output.trim();
}

function city(name) {
  return name.split(/,| \(/, 1)[0];
}

function key(region) {
  return region.city.name
    .replace(" de ", " De ")
    .replace(/\s+/g, "")
    .replace("á", "a")
    .replace("é", "e");
}

function country(name) {
  const countries = {
    Argentina: "AR",
    Australia: "AU",
    Brazil: "BR",
    Canada: "CA",
    Chile: "CL",
    Colombia: "CO",
    France: "FR",
    Germany: "DE",
    "Hong Kong": "HK",
    India: "IN",
    Japan: "JP",
    Mexico: "MX",
    Netherlands: "NL",
    Poland: "PL",
    Romania: "RO",
    Singapore: "SG",
    "South Africa": "ZA",
    Spain: "ES",
    Sweden: "SE",
    "United Kingdom": "GB",
  };

  return name.endsWith(" (US)") ? "US" : countries[name.split(", ", 2).pop()];
}

function hex(s) {
  return [...s]
    .map((c) => c.charCodeAt(0).toString(16).padStart(2, "0"))
    .join("");
}

function reverse(s) {
  return [...s].reverse().join("");
}

function repr(v) {
  return JSON.stringify(v);
}
