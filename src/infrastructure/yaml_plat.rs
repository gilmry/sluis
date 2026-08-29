//! Lecture d'un sous-ensemble de YAML, aplati en chemins de clés.
//!
//! **Pourquoi pas une bibliothèque YAML complète.** Sluis ne lit du YAML que
//! pour un contrat connu et court : les cinq clés d'un profil de cluster. Une
//! bibliothèque généraliste apporterait ici un arbre de dépendances et une
//! surface d'attaque hors de proportion avec le besoin, ce que la sobriété de
//! la méthode déconseille.
//!
//! **Ce que ce module ne sait pas faire, et l'assume** : ancres et références,
//! documents multiples, blocs littéraux `|` et `>`, listes complexes. Un profil
//! qui en userait serait mal lu — c'est pourquoi les valeurs manquantes sont
//! rendues absentes plutôt que devinées, et pourquoi les listes sont ignorées
//! au lieu d'être approximées.

use std::collections::BTreeMap;

/// Aplatit un YAML en chemins pointés, par exemple `global.tls.enabled`.
///
/// Rend une erreur lisible plutôt qu'un résultat partiel silencieux : un profil
/// mal formé doit se voir, sinon Sluis affirmerait un contrat de cluster faux.
pub fn aplatir(contenu: &str) -> Result<BTreeMap<String, String>, String> {
    let mut plat = BTreeMap::new();
    // Pile des (indentation, clé) ouverts.
    let mut pile: Vec<(usize, String)> = Vec::new();

    for (numero, ligne_brute) in contenu.lines().enumerate() {
        let sans_commentaire = retirer_commentaire(ligne_brute);
        if sans_commentaire.trim().is_empty() {
            continue;
        }
        if sans_commentaire.starts_with('\t')
            || sans_commentaire
                .chars()
                .take_while(|c| c.is_whitespace())
                .any(|c| c == '\t')
        {
            return Err(format!(
                "ligne {} : l'indentation par tabulation est interdite en YAML",
                numero + 1
            ));
        }
        let indentation = sans_commentaire.len() - sans_commentaire.trim_start().len();
        let contenu_ligne = sans_commentaire.trim();

        // Les éléments de liste sont hors du sous-ensemble traité.
        if contenu_ligne.starts_with('-') {
            continue;
        }
        // Les séparateurs de document aussi.
        if contenu_ligne == "---" || contenu_ligne == "..." {
            continue;
        }

        let Some((cle, valeur)) = contenu_ligne.split_once(':') else {
            return Err(format!(
                "ligne {} : « {contenu_ligne} » n'est ni une clé ni une valeur",
                numero + 1
            ));
        };
        let cle = cle.trim();
        if cle.is_empty() {
            return Err(format!("ligne {} : clé vide", numero + 1));
        }

        while pile.last().is_some_and(|(i, _)| *i >= indentation) {
            pile.pop();
        }

        let valeur = valeur.trim();
        if valeur.is_empty() {
            pile.push((indentation, cle.to_string()));
        } else {
            let mut chemin: Vec<&str> = pile.iter().map(|(_, c)| c.as_str()).collect();
            chemin.push(cle);
            plat.insert(chemin.join("."), denuder(valeur).to_string());
        }
    }
    Ok(plat)
}

/// Retire un commentaire de fin de ligne, sans casser une valeur entre guillemets.
fn retirer_commentaire(ligne: &str) -> &str {
    let mut dans_guillemets = false;
    let mut quote = '"';
    for (index, caractere) in ligne.char_indices() {
        match caractere {
            '"' | '\'' if !dans_guillemets => {
                dans_guillemets = true;
                quote = caractere;
            }
            c if dans_guillemets && c == quote => dans_guillemets = false,
            // Un `#` collé à du texte ne commence pas un commentaire : la
            // condition est portée par la garde, pour que le cas « # au milieu
            // d'un mot » reste visiblement un non-cas.
            '#' if !dans_guillemets
                && (index == 0 || ligne[..index].ends_with(char::is_whitespace)) =>
            {
                return &ligne[..index];
            }
            _ => {}
        }
    }
    ligne
}

/// Retire les guillemets encadrants d'une valeur scalaire.
fn denuder(valeur: &str) -> &str {
    let bytes = valeur.as_bytes();
    if valeur.len() >= 2
        && ((bytes[0] == b'"' && bytes[valeur.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[valeur.len() - 1] == b'\''))
    {
        &valeur[1..valeur.len() - 1]
    } else {
        valeur
    }
}
