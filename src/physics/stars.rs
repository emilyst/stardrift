use crate::resources::BodyCount;
use crate::resources::SharedRng;
use crate::utils::color::emissive_material_for_temp;
use crate::utils::math::min_sphere_radius_for_surface_distribution;
use crate::utils::math::random_unit_vector;
use avian3d::math::Scalar;
use avian3d::prelude::*;
use bevy::prelude::*;
use libm::pow;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

#[derive(Resource, Default)]
struct StellarParameters {
    solar_mass: Scalar,
    solar_radius: Scalar,
    solar_luminosity: Scalar,
    solar_temperature: Scalar,
}

impl StellarParameters {
    fn new() -> Self {
        Self {
            solar_mass: 1.0,
            solar_radius: 10.0,
            solar_luminosity: 1.0,
            solar_temperature: 5778.0,
        }
    }
}

#[derive(Component)]
struct StellarProperties {
    mass_solar: Scalar,
    radius_solar: Scalar,
    luminosity_solar: Scalar,
    temperature: Scalar,
    stellar_class: StellarClass,
}

#[derive(Debug, Clone)]
enum StellarClass {
    O,
    B,
    A,
    F,
    G,
    K,
    M,
}

impl StellarClass {
    fn from_temperature(temp: Scalar) -> Self {
        match temp {
            t if t >= 30000.0 => StellarClass::O,
            t if t >= 10000.0 => StellarClass::B,
            t if t >= 7500.0 => StellarClass::A,
            t if t >= 6000.0 => StellarClass::F,
            t if t >= 5200.0 => StellarClass::G,
            t if t >= 3700.0 => StellarClass::K,
            _ => StellarClass::M,
        }
    }
}

// Salpeter Initial Mass Function (simplified)
fn sample_stellar_mass_salpeter(rng: &mut ChaCha8Rng) -> Scalar {
    let alpha = 2.35; // Salpeter slope
    let min_mass = 0.1; // Minimum mass in solar masses
    let max_mass = 100.0; // Maximum mass in solar masses

    let u: f64 = rng.random();
    let exponent = 1.0 - alpha;

    let mass = pow(
        pow(min_mass, exponent) + u * (pow(max_mass, exponent) - pow(min_mass, exponent)),
        1.0 / exponent,
    );

    mass as Scalar
}

// More realistic Kroupa IMF (broken power law)
fn sample_stellar_mass_kroupa(rng: &mut ChaCha8Rng) -> Scalar {
    let u: f64 = rng.random();

    // Kroupa IMF has different slopes for different mass ranges
    if u < 0.08 {
        // Brown dwarfs and very low mass stars (0.01 - 0.08 M_sun)
        let alpha = 0.3;
        let min_mass = 0.01;
        let max_mass = 0.08;
        let v: f64 = rng.random();
        min_mass * pow(max_mass / min_mass, v * alpha)
    } else if u < 0.5 {
        // Low mass stars (0.08 - 0.5 M_sun)
        let alpha = 1.3;
        let min_mass = 0.08;
        let max_mass = 0.5;
        let v: f64 = rng.random();
        let exponent = 1.0 - alpha;
        pow(
            pow(min_mass, exponent) + v * (pow(max_mass, exponent) - pow(min_mass, exponent)),
            1.0 / exponent,
        )
    } else {
        // Higher mass stars (0.5 - 100 M_sun)
        let alpha = 2.3;
        let min_mass = 0.5;
        let max_mass = 100.0;
        let v: f64 = rng.random();
        let exponent = 1.0 - alpha;
        pow(
            pow(min_mass, exponent) + v * (pow(max_mass, exponent) - pow(min_mass, exponent)),
            1.0 / exponent,
        )
    }
}

// Calculate stellar properties from mass using empirical relations
fn calculate_stellar_properties(mass_solar: Scalar) -> (Scalar, Scalar, Scalar) {
    // Mass-luminosity relation (main sequence)
    let luminosity_solar = if mass_solar < 0.43 {
        0.23 * pow(mass_solar, 2.3)
    } else if mass_solar < 2.0 {
        pow(mass_solar, 4.0)
    } else if mass_solar < 55.0 {
        1.4 * pow(mass_solar, 3.5)
    } else {
        32000.0 * mass_solar
    };

    // Mass-temperature relation (main sequence)
    let temperature = if mass_solar < 1.0 {
        5778.0 * pow(mass_solar, 0.6) // Cooler stars
    } else {
        5778.0 * pow(mass_solar, 0.5) // Hotter stars
    };

    // Calculate radius from Stefan-Boltzmann law: L = 4πR²σT⁴
    // R = sqrt(L / (4π σ T⁴)) in solar units
    // let stefan_boltzmann = 5.67e-8; // Not needed for ratio calculation
    let temp_ratio_4th = pow(temperature / 5778.0, 4.0);
    let radius_solar = libm::sqrt(luminosity_solar / temp_ratio_4th);

    (radius_solar, luminosity_solar, temperature)
}

fn apply_stellar_evolution(
    mass_solar: Scalar,
    age_gyr: Scalar,
    rng: &mut ChaCha8Rng,
) -> (Scalar, Scalar, Scalar, bool) {
    let main_sequence_lifetime = match mass_solar {
        m if m > 15.0 => 0.01, // Very massive stars live ~10 Myr
        m if m > 8.0 => 0.03,  // Massive stars live ~30 Myr
        m if m > 3.0 => 0.5,   // Medium stars live ~500 Myr
        m if m > 1.5 => 2.0,   // Sun-like stars live ~2 Gyr
        m if m > 1.0 => 10.0,  // Lower mass stars live ~10 Gyr
        _ => 100.0,            // Red dwarfs live >100 Gyr
    };

    let is_evolved = age_gyr > main_sequence_lifetime;

    if !is_evolved {
        // Still on main sequence
        let (radius, luminosity, temperature) = calculate_stellar_properties(mass_solar);
        (radius, luminosity, temperature, false)
    } else {
        // Post-main sequence evolution (simplified)
        if mass_solar > 8.0 {
            // Massive stars → supergiants or already exploded
            let prob_exploded = (age_gyr - main_sequence_lifetime) / 0.1;
            if rng.random::<f64>() < prob_exploded {
                // Star has exploded, might be neutron star or black hole
                (0.01, 0.001, 1000.0, true) // Compact remnant
            } else {
                // Red/blue supergiant
                let (base_radius, base_luminosity, base_temp) =
                    calculate_stellar_properties(mass_solar);
                (
                    base_radius * 50.0,
                    base_luminosity * 10.0,
                    base_temp * 0.6,
                    true,
                )
            }
        } else if mass_solar > 0.5 {
            // Lower mass stars → red giants → white dwarfs
            let evolution_phase = (age_gyr - main_sequence_lifetime) / 2.0;
            if evolution_phase < 1.0 {
                // Red giant branch
                let (base_radius, base_luminosity, base_temp) =
                    calculate_stellar_properties(mass_solar);
                let expansion_factor = 1.0 + evolution_phase * 20.0;
                (
                    base_radius * expansion_factor,
                    base_luminosity * 2.0,
                    base_temp * 0.7,
                    true,
                )
            } else {
                // White dwarf
                (0.01, 0.001, 10000.0, true)
            }
        } else {
            // Very low mass stars don't evolve significantly
            let (radius, luminosity, temperature) = calculate_stellar_properties(mass_solar);
            (radius, luminosity, temperature, false)
        }
    }
}

pub(crate) fn spawn_realistic_stellar_bodies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<SharedRng>,
    body_count: Res<BodyCount>,
) {
    let stellar_params = StellarParameters::new();
    let stellar_population_max_age_gyr = 10.0; // Assume 10 billion year old stellar population

    for _ in 0..**body_count {
        let body_distribution_sphere_radius =
            min_sphere_radius_for_surface_distribution(**body_count, 1000.0, 0.001);
        let position = random_unit_vector(&mut rng) * body_distribution_sphere_radius;
        let transform = Transform::from_translation(position.as_vec3());

        let mass_solar = sample_stellar_mass_kroupa(&mut rng);

        let age_gyr = rng.random_range(0.1..stellar_population_max_age_gyr);

        let (radius_solar, luminosity_solar, temperature, is_evolved) =
            apply_stellar_evolution(mass_solar, age_gyr, &mut rng);

        let radius_sim = radius_solar * stellar_params.solar_radius;
        let mass_sim = mass_solar * stellar_params.solar_mass;

        let stellar_props = StellarProperties {
            mass_solar,
            radius_solar,
            luminosity_solar,
            temperature,
            stellar_class: StellarClass::from_temperature(temperature),
        };

        let mesh = meshes.add(Sphere::new(radius_sim as f32));

        let bloom_intensity = (luminosity_solar * 100.0).clamp(100.0, 10000.0);
        let saturation_intensity = if is_evolved { 1.0 } else { 2.0 };
        let material = emissive_material_for_temp(
            &mut materials,
            temperature,
            bloom_intensity,
            saturation_intensity,
        );

        commands.spawn((
            Name::new(format!("Star-{mass_solar:.2}M☉-{temperature:.0}K")),
            transform,
            Collider::sphere(radius_sim),
            GravityScale(0.0),
            RigidBody::Dynamic,
            MeshMaterial3d(material.clone()),
            Mesh3d(mesh),
            stellar_props,
            Mass(mass_sim as f32),
        ));
    }
}
