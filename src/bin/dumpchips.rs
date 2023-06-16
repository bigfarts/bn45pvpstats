use tango_dataview::rom::Assets;

#[derive(serde::Deserialize, Debug)]
struct PatchMetadata {
    pub versions: std::collections::HashMap<String, VersionMetadata>,
}

#[derive(serde::Deserialize, Debug)]
struct RomOverrides {
    pub charset: Vec<String>,
}

#[derive(serde::Deserialize, Debug)]
struct VersionMetadata {
    pub rom_overrides: RomOverrides,
}

#[derive(serde::Serialize, Debug)]
struct Chips {
    pub names: Vec<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let rom = std::fs::read("exe45.gba")?;

    let us_assets = {
        let patch_metadata =
            toml::from_slice::<PatchMetadata>(&std::fs::read("patches/bn45_us_pvp/info.toml")?)?;
        tango_dataview::game::exe45::rom::Assets::new(
            &tango_dataview::game::exe45::rom::BR4J_00,
            &patch_metadata
                .versions
                .get("0.6.0")
                .unwrap()
                .rom_overrides
                .charset,
            bps::Patch::decode(&std::fs::read("patches/bn45_us_pvp/v0.6.0/BR4J_00.bps")?)?
                .apply(&rom)?,
            vec![0; 65536],
        )
    };

    let ja_assets = {
        tango_dataview::game::exe45::rom::Assets::new(
            &tango_dataview::game::exe45::rom::BR4J_00,
            &tango_dataview::game::exe45::rom::CHARSET
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            bps::Patch::decode(&std::fs::read("patches/exe45_pvp/v0.6.0/BR4J_00.bps")?)?
                .apply(&rom)?,
            vec![0; 65536],
        )
    };

    let us_chip_names = (0..us_assets.num_chips())
        .map(|i| us_assets.chip(i).unwrap().name().unwrap())
        .collect::<Vec<_>>();
    std::fs::write(
        "reportgen/locales/en/chips.toml",
        toml::to_string_pretty(&Chips {
            names: us_chip_names,
        })?,
    )?;

    let ja_chip_names = (0..ja_assets.num_chips())
        .map(|i| ja_assets.chip(i).unwrap().name().unwrap())
        .collect::<Vec<_>>();
    std::fs::write(
        "reportgen/locales/ja/chips.toml",
        toml::to_string_pretty(&Chips {
            names: ja_chip_names,
        })?,
    )?;

    for i in 0..us_assets.num_chips() {
        let icon = us_assets.chip(i).unwrap().icon();
        let icon = image::imageops::crop_imm(&icon, 1, 1, 14, 14).to_image();
        icon.save(format!("reportgen/images/chips/{}.png", i))?;
    }

    Ok(())
}
