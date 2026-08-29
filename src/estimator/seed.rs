//! Seeds for the estimator tables, ported from the legacy
//! `EstimatorAttributeSeeder` and `EstimatorStatisticSeeder`. Both run at
//! the end of `sde_import` (the legacy runs them via migrations and
//! `db:seed`) and are idempotent.

use sqlx::PgPool;

/// The legacy `EstimatorAttributeSeeder` groups: per family of abyssal
/// types, the dogma attributes their value models are trained on. Names
/// are verbatim from the seeder — including its lowercase `5mn`/`50mn`
/// spellings, which only match the real `5MN`/`50MN` type names because
/// legacy MySQL compares case-insensitively; the seeding query lowercases
/// both sides to mirror that.
const FEATURE_GROUPS: &[(&[&str], &[&str])] = &[
    // Microwarpdrives
    (
        &[
            "5mn Abyssal Microwarpdrive",
            "50mn Abyssal Microwarpdrive",
            "500mn Abyssal Microwarpdrive",
            "50000mn Abyssal Microwarpdrive",
        ],
        &[
            "capacitorNeed",
            "cpu",
            "speedFactor",
            "capacitorCapacityMultiplier",
            "signatureRadiusBonus",
            "power",
            "overloadSpeedFactorBonus",
        ],
    ),
    // Turret damage mods
    (
        &[
            "Abyssal Heat Sink",
            "Abyssal Gyrostabilizer",
            "Abyssal Magnetic Field Stabilizer",
            "Abyssal Entropic Radiation Sink",
            "Abyssal Vorton Tuning System",
        ],
        &["cpu", "damageMultiplier", "speedMultiplier"],
    ),
    // Ballistic control systems
    (
        &["Abyssal Ballistic Control System"],
        &[
            "cpu",
            "missileDamageMultiplierBonus",
            "speedMultiplier",
            "droneDamageBonus",
            "aoeVelocityBonus",
        ],
    ),
    // Shield boosters
    (
        &[
            "Small Abyssal Shield Booster",
            "Medium Abyssal Shield Booster",
            "Large Abyssal Shield Booster",
            "X-Large Abyssal Shield Booster",
            "Capital Abyssal Shield Booster",
        ],
        &[
            "cpu",
            "shieldBonus",
            "power",
            "duration",
            "capacitorNeed",
            "heatDamage",
        ],
    ),
    // Armor repairers
    (
        &[
            "Small Abyssal Armor Repairer",
            "Medium Abyssal Armor Repairer",
            "Large Abyssal Armor Repairer",
            "Capital Abyssal Armor Repairer",
        ],
        &[
            "cpu",
            "armorDamageAmount",
            "power",
            "duration",
            "capacitorNeed",
            "heatDamage",
        ],
    ),
    // Warp disruptors
    (
        &["Abyssal Warp Disruptor"],
        &["cpu", "capacitorNeed", "maxRange"],
    ),
    // Heavy warp disruptors
    (
        &["Heavy Abyssal Warp Disruptor"],
        &[
            "cpu",
            "capacitorNeed",
            "maxRange",
            "warpScrambleStrength",
            "power",
        ],
    ),
    // Warp scramblers
    (
        &["Abyssal Warp Scrambler"],
        &["cpu", "capacitorNeed", "maxRange", "warpScrambleStrength"],
    ),
    // Heavy warp scramblers
    (
        &["Heavy Abyssal Warp Scrambler"],
        &[
            "cpu",
            "capacitorNeed",
            "maxRange",
            "warpScrambleStrength",
            "power",
        ],
    ),
    // Stasis webifiers
    (
        &["Abyssal Stasis Webifier"],
        &["cpu", "capacitorNeed", "maxRange", "speedFactor", "power"],
    ),
    // Drone damage amplifiers
    (
        &["Mutated Drone Damage Amplifier"],
        &["cpu", "droneDamageBonus", "power"],
    ),
    // Siege modules
    (
        &["Abyssal Siege Module"],
        &[
            "cpu",
            "power",
            "siegeMissileDamageBonus",
            "siegeTurretDamageBonus",
            "sensorDampenerResistanceBonus",
            "weaponDisruptionResistanceBonus",
            "siegeLocalLogisticsAmountBonus",
            "siegeLocalLogisticsDurationBonus",
        ],
    ),
    // Fighter support units
    (
        &["Mutated Fighter Support Unit"],
        &[
            "cpu",
            "power",
            "fighterBonusShieldCapacityPercent",
            "fighterBonusVelocityPercent",
            "fighterBonusROFPercent",
            "fighterBonusShieldRechargePercent",
        ],
    ),
    // Ancillary shield boosters
    (
        &[
            "Medium Abyssal Ancillary Shield Booster",
            "Large Abyssal Ancillary Shield Booster",
            "X-Large Abyssal Ancillary Shield Booster",
            "Capital Abyssal Ancillary Shield Booster",
        ],
        &[
            "cpu",
            "shieldBonus",
            "power",
            "duration",
            "capacitorNeed",
            "reloadTime",
        ],
    ),
    // Shield extenders
    (
        &[
            "Small Abyssal Shield Extender",
            "Medium Abyssal Shield Extender",
            "Large Abyssal Shield Extender",
        ],
        &["cpu", "power", "capacityBonus", "signatureRadiusAdd"],
    ),
    // Ancillary armor repairers
    (
        &[
            "Small Abyssal Ancillary Armor Repairer",
            "Medium Abyssal Ancillary Armor Repairer",
            "Large Abyssal Ancillary Armor Repairer",
            "Capital Abyssal Ancillary Armor Repairer",
        ],
        &[
            "cpu",
            "armorDamageAmount",
            "power",
            "duration",
            "capacitorNeed",
            "reloadTime",
        ],
    ),
    // Armor plates
    (
        &[
            "Small Abyssal Armor Plates",
            "Medium Abyssal Armor Plates",
            "Large Abyssal Armor Plates",
        ],
        &["cpu", "power", "armorHpBonusAdd", "massAddition"],
    ),
    // Damage controls
    (
        &["Abyssal Damage Control"],
        &[
            "cpu",
            "armorEmDamageResonance",
            "armorExplosiveDamageResonance",
            "armorKineticDamageResonance",
            "armorThermalDamageResonance",
            "shieldEmDamageResonance",
            "shieldExplosiveDamageResonance",
            "shieldKineticDamageResonance",
            "shieldThermalDamageResonance",
            "structureEmDamageResonance",
            "structureExplosiveDamageResonance",
            "structureKineticDamageResonance",
            "structureThermalDamageResonance",
        ],
    ),
    // Assault damage controls
    (
        &["Abyssal Assault Damage Control"],
        &[
            "cpu",
            "duration",
            "armorEmDamageResonance",
            "armorExplosiveDamageResonance",
            "armorKineticDamageResonance",
            "armorThermalDamageResonance",
            "shieldEmDamageResonance",
            "shieldExplosiveDamageResonance",
            "shieldKineticDamageResonance",
            "shieldThermalDamageResonance",
            "structureEmDamageResonance",
            "structureExplosiveDamageResonance",
            "structureKineticDamageResonance",
            "structureThermalDamageResonance",
        ],
    ),
    // Afterburners
    (
        &[
            "1mn Abyssal Afterburner",
            "10mn Abyssal Afterburner",
            "100mn Abyssal Afterburner",
            "10000mn Abyssal Afterburner",
        ],
        &[
            "capacitorNeed",
            "cpu",
            "speedFactor",
            "power",
            "overloadSpeedFactorBonus",
        ],
    ),
    // Energy neutralizers
    (
        &[
            "Small Abyssal Energy Neutralizer",
            "Medium Abyssal Energy Neutralizer",
            "Heavy Abyssal Energy Neutralizer",
            "Capital Abyssal Energy Neutralizer",
        ],
        &[
            "cpu",
            "capacitorNeed",
            "power",
            "energyNeutralizerAmount",
            "maxRange",
            "heatAbsorbtionRateModifier",
            "entityCapacitorLevelModifierMedium",
            "entityCapacitorLevelModifierLarge",
            "falloffEffectiveness",
        ],
    ),
    // Energy nosferatus
    (
        &[
            "Small Abyssal Energy Nosferatu",
            "Medium Abyssal Energy Nosferatu",
            "Heavy Abyssal Energy Nosferatu",
            "Capital Abyssal Energy Nosferatu",
        ],
        &[
            "cpu",
            "capacitorNeed",
            "power",
            "powerTransferAmount",
            "maxRange",
            "heatAbsorbtionRateModifier",
            "falloffEffectiveness",
        ],
    ),
    // Cap batteries
    (
        &[
            "Small Abyssal Cap Battery",
            "Medium Abyssal Cap Battery",
            "Large Abyssal Cap Battery",
        ],
        &[
            "capacitorBonus",
            "cpu",
            "power",
            "energyWarfareResistanceBonus",
        ],
    ),
    // Combat drones
    (
        &[
            "Light Mutated Drone",
            "Medium Mutated Drone",
            "Heavy Mutated Drone",
            "Sentry Mutated Drone",
        ],
        &[
            "armorHp",
            "damageMultiplier",
            "falloff",
            "hp",
            "maxRange",
            "shieldCapacity",
            "trackingSpeed",
            "velocity",
            "emDamage",
            "explosiveDamage",
            "kineticDamage",
            "entityCruiseSpeed",
        ],
    ),
    // EMP smartbombs
    (
        &[
            "Small Abyssal EMP Smartbomb",
            "Medium Abyssal EMP Smartbomb",
            "Large Abyssal EMP Smartbomb",
        ],
        &[
            "cpu",
            "power",
            "capacitorNeed",
            "duration",
            "empFieldRange",
            "emDamage",
        ],
    ),
    // Plasma smartbombs
    (
        &[
            "Small Abyssal Plasma Smartbomb",
            "Medium Abyssal Plasma Smartbomb",
            "Large Abyssal Plasma Smartbomb",
        ],
        &[
            "cpu",
            "power",
            "capacitorNeed",
            "duration",
            "empFieldRange",
            "thermalDamage",
        ],
    ),
    // Graviton smartbombs
    (
        &[
            "Small Abyssal Graviton Smartbomb",
            "Medium Abyssal Graviton Smartbomb",
            "Large Abyssal Graviton Smartbomb",
        ],
        &[
            "cpu",
            "power",
            "capacitorNeed",
            "duration",
            "empFieldRange",
            "kineticDamage",
        ],
    ),
    // Proton smartbombs
    (
        &[
            "Small Abyssal Proton Smartbomb",
            "Medium Abyssal Proton Smartbomb",
            "Large Abyssal Proton Smartbomb",
        ],
        &[
            "cpu",
            "power",
            "capacitorNeed",
            "duration",
            "empFieldRange",
            "explosiveDamage",
        ],
    ),
    // Mining lasers
    (
        &[
            "Abyssal Mining Laser",
            "Abyssal Deep Core Mining Laser",
            "Abyssal Modulated Deep Core Miner",
        ],
        &[
            "capacitorNeed",
            "cpu",
            "duration",
            "effectiveMiningSpeed",
            "maxRange",
            "miningAmount",
            "miningCritBonusYield",
            "miningCritChance",
            "miningWastedVolumeMultiplier",
            "miningWasteProbability",
            "power",
        ],
    ),
    // Strip miners
    (
        &[
            "Abyssal Strip Miner",
            "Abyssal Deep Core Strip Miner",
            "Abyssal Modulated Strip Miner",
            "Abyssal Modulated Deep Core Strip Miner",
        ],
        &[
            "capacitorNeed",
            "cpu",
            "duration",
            "effectiveMiningSpeed",
            "maxRange",
            "miningAmount",
            "miningCritBonusYield",
            "miningCritChance",
            "miningWastedVolumeMultiplier",
            "miningWasteProbability",
            "power",
        ],
    ),
    // Ice mining modules
    (
        &["Abyssal Ice Mining Laser", "Abyssal Ice Harvester"],
        &[
            "capacitorNeed",
            "cpu",
            "duration",
            "effectiveMiningSpeed",
            "maxRange",
            "miningAmount",
            "miningCritBonusYield",
            "miningCritChance",
            "miningWastedVolumeMultiplier",
            "miningWasteProbability",
            "power",
        ],
    ),
    // Gas harvesting modules
    (
        &["Abyssal Gas Cloud Scoop", "Abyssal Gas Cloud Harvester"],
        &[
            "capacitorNeed",
            "cpu",
            "duration",
            "maxRange",
            "miningAmount",
            "miningSpeed",
            "miningWastedVolumeMultiplier",
            "miningWasteProbability",
            "power",
        ],
    ),
    // Mining drones
    (
        &[
            "Mutated Mining Drone",
            "Mutated Ice Harvesting Drone",
            "Mutated 'Excavator' Mining Drone",
            "Mutated 'Excavator' Ice Harvesting Drone",
        ],
        &[
            "duration",
            "maxRange",
            "maxVelocity",
            "miningAmount",
            "miningSpeed",
            "miningWastedVolumeMultiplier",
            "miningWasteProbability",
        ],
    ),
];

/// Rebuilds `estimator_attributes` from [`FEATURE_GROUPS`], like the legacy
/// seeder's truncate + firstOrCreate. Type and attribute names are matched
/// case-insensitively because legacy MySQL collates case-insensitively.
pub async fn seed_estimator_attributes(pool: &PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query("delete from estimator_attributes")
        .execute(&mut *tx)
        .await?;

    for (type_names, attribute_names) in FEATURE_GROUPS {
        let type_names: Vec<String> = type_names.iter().map(|name| name.to_lowercase()).collect();
        let attribute_names: Vec<String> = attribute_names
            .iter()
            .map(|name| name.to_lowercase())
            .collect();

        sqlx::query(
            "insert into estimator_attributes (type_id, attribute_id)
             select t.id, a.id
             from types t
             cross join attributes a
             where lower(t.name) = any($1) and lower(a.name) = any($2)
             on conflict (type_id, attribute_id) do nothing",
        )
        .bind(&type_names)
        .bind(&attribute_names)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await
}

/// Creates the baseline `estimator_statistics` rows, like the legacy
/// `EstimatorStatisticSeeder`: one row per mutaplasmid output type with
/// zero counts, a null (untrained) `r2`, and `data_statistics` prefilled
/// with a zero per meta group occurring among the type's mutaplasmid input
/// types. Existing rows — with their trained metrics — are left untouched
/// (firstOrCreate).
pub async fn seed_estimator_statistics(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query(
        "insert into estimator_statistics (type_id, name, data_count, data_statistics)
         select t.id, t.name, 0, coalesce(
             (
                 select jsonb_object_agg(mg.name, 0)
                 from meta_groups mg
                 where mg.id in (
                     select input.meta_group_id
                     from types input
                     join mutaplasmid_input_types mit on mit.type_id = input.id
                     join mutaplasmids m on m.id = mit.mutaplasmid_id
                     where m.output_type_id = t.id and input.meta_group_id is not null
                 )
             ),
             '{}'::jsonb
         )
         from types t
         where t.id in (select distinct output_type_id from mutaplasmids)
         on conflict (type_id) do nothing",
    )
    .execute(pool)
    .await?;

    Ok(())
}
