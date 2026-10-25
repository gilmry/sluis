//! La section `verification-empreinte` du workflow de Tier 1 (ADR-034, point 6).
//!
//! Le premier pas du workflow cible recalcule l'empreinte du plan à partir
//! des entrées du dispatch et s'arrête si elle ne correspond pas : c'est ce qui
//! lie une approbation au plan approuvé (faille S2). Une cible qui affaiblirait
//! ce pas, même d'un caractère, rendrait l'approbation transférable. Sluis
//! compare donc la section de la cible à celle du gabarit avant chaque
//! dispatch.
//!
//! **Ce qui est comparé** : les lignes entre les deux marqueurs, à
//! l'identique, l'indentation de tête seule étant ignorée. Elle tient à la
//! structure YAML du fichier, pas au script exécuté.
//!
//! **Chaque marqueur doit apparaître exactement une fois.** Une seconde
//! section, intacte, placée ailleurs ne doit pas masquer une vraie section
//! modifiée.
//!
//! **Limite assumée** (ADR-034) : prouver que ce bloc est intact ne prouve pas
//! que le reste du workflow est sain.

/// Marqueur d'ouverture de la section.
pub const DEBUT_SECTION: &str = "# >>> verification-empreinte";

/// Marqueur de fermeture de la section.
pub const FIN_SECTION: &str = "# <<< verification-empreinte";

/// Extrait la section, marqueurs compris, lignes désindentées.
pub fn extraire_section(workflow: &str) -> Result<Vec<&str>, String> {
    let lignes: Vec<&str> = workflow.lines().map(str::trim_start).collect();
    let position = |marqueur: &str| -> Result<usize, String> {
        let trouvees: Vec<usize> = lignes
            .iter()
            .enumerate()
            .filter(|(_, ligne)| ligne.contains(marqueur))
            .map(|(index, _)| index)
            .collect();
        match trouvees.as_slice() {
            [unique] => Ok(*unique),
            [] => Err(format!("marqueur « {marqueur} » absent")),
            _ => Err(format!(
                "marqueur « {marqueur} » présent {} fois, une seule section est admise",
                trouvees.len()
            )),
        }
    };
    let debut = position(DEBUT_SECTION)?;
    let fin = position(FIN_SECTION)?;
    if fin < debut {
        return Err("le marqueur de fin précède le marqueur de début".to_string());
    }
    Ok(lignes[debut..=fin].to_vec())
}

/// Vérifie que la section du workflow cible est celle du gabarit.
///
/// L'erreur nomme la première ligne qui diffère, numérotée depuis le marqueur
/// d'ouverture : c'est ce que le superviseur doit corriger dans la cible.
pub fn verifier_section(gabarit: &str, cible: &str) -> Result<(), String> {
    let attendue =
        extraire_section(gabarit).map_err(|e| format!("gabarit de Sluis illisible : {e}"))?;
    let lue = extraire_section(cible)?;
    for (index, (a, l)) in attendue.iter().zip(lue.iter()).enumerate() {
        if a != l {
            return Err(format!(
                "section modifiée à la ligne {} de la section : attendu « {a} », lu « {l} »",
                index + 1
            ));
        }
    }
    if attendue.len() != lue.len() {
        return Err(format!(
            "section de {} lignes, le gabarit en compte {}",
            lue.len(),
            attendue.len()
        ));
    }
    Ok(())
}
