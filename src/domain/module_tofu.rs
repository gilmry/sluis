//! Vérification d'un module OpenTofu avant de le lancer (ADR-034, point 3).
//!
//! **Un fichier `.tf` est un programme, pas une donnée.** Un
//! `provisioner "local-exec"` ou un `data "external"` exécute une commande
//! arbitraire sur la machine qui lance `tofu`, avec ses secrets, et
//! `data "external"` le fait dès le `plan`. Ce module dit, sans rien lire du
//! disque ni rien lancer, si un module sort du contrat.
//!
//! Trois décisions le structurent.
//!
//! **Le vérificateur est pur.** Il reçoit des chemins et des contenus, il rend
//! des violations. La lecture des fichiers appartient à un adaptateur ; ici,
//! tout se teste avec des chaînes.
//!
//! **Il lit le texte brut, commentaires compris.** Une construction interdite
//! écrite dans un commentaire est refusée : c'est un faux positif assumé,
//! jamais un oubli. Écrire un analyseur HCL complet pour distinguer le code du
//! commentaire serait plus de surface que ce que la règle protège. Les blancs,
//! retours à la ligne et commentaires entre le mot-clé et son étiquette sont
//! sautés, pour qu'aucune mise en forme ne fasse passer une construction.
//!
//! **Tout ce qui n'est pas lisible est refusé.** Une étiquette ou une source
//! qui n'est pas un littéral simple (séquence d'échappement, interpolation,
//! variable) ne peut pas être jugée sans évaluer le langage : elle est refusée
//! comme telle, plutôt que de supposer qu'elle est sage.

use std::fmt;

/// Nom du fichier de verrouillage des fournisseurs, à la racine du module.
pub const FICHIER_VERROU: &str = ".terraform.lock.hcl";

/// Le seul endroit d'où un bloc `module` peut tirer sa source : décision du
/// superviseur du 2026-10-23, un module est copié dans le dépôt.
pub const MODULES_PARTAGES: &[&str] = &["_shared", "terraform", "modules"];

/// Extensions des programmes OpenTofu que le vérificateur lit.
pub const EXTENSIONS_PROGRAMME: &[&str] = &[".tf", ".tofu"];

/// Extensions de la syntaxe JSON, refusées : le vérificateur ne les lit pas.
pub const EXTENSIONS_JSON_REFUSEES: &[&str] = &[".tf.json", ".tofu.json"];

/// Les constructions qui exécutent une commande chez l'exécutant.
///
/// **C'est la liste que le témoin de la story 34.1 ampute** : en retirer une
/// ligne doit faire échouer le test `@negative` correspondant.
const INTERDITS: &[(&str, &str, Construction)] = &[
    (
        "provisioner",
        "local-exec",
        Construction::ProvisionerLocalExec,
    ),
    (
        "provisioner",
        "remote-exec",
        Construction::ProvisionerRemoteExec,
    ),
    ("data", "external", Construction::DataExternal),
];

/// Les constructions interdites, écrites comme dans un fichier `.tf`, par
/// exemple `provisioner "local-exec"`. C'est ce que publie le contrat du
/// dépôt cible (ADR-034) : la même liste que celle qui est vérifiée.
pub fn constructions_interdites() -> Vec<String> {
    INTERDITS
        .iter()
        .map(|(bloc, etiquette, _)| format!("{bloc} \"{etiquette}\""))
        .collect()
}

/// Un fichier du module soumis, tel que l'adaptateur l'a lu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FichierModule {
    /// Chemin relatif à la racine du dépôt, séparé par `/`.
    pub chemin: String,
    /// Contenu texte.
    pub contenu: String,
}

/// Un module soumis à la vérification.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModuleSoumis {
    /// Dossier du module racine, relatif à la racine du dépôt (`""` pour la
    /// racine elle-même). C'est là que `tofu` est lancé, donc là que le
    /// fichier de verrouillage est exigé.
    pub chemin: String,
    /// Chemin absolu **canonique** du module, tel que le lecteur l'a résolu
    /// et accepté. C'est le dossier où `tofu` est lancé : pas le chemin
    /// soumis, qui peut passer par un lien symbolique (ADR-034, point 4).
    /// Vide, rien n'est lancé.
    pub dossier: String,
    /// Les fichiers à juger : ceux du module racine, et ceux des modules
    /// partagés qu'il peut appeler.
    pub fichiers: Vec<FichierModule>,
}

/// Ce qui est refusé. Un motif distinct par cas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Construction {
    /// `provisioner "local-exec"`.
    ProvisionerLocalExec,
    /// `provisioner "remote-exec"`.
    ProvisionerRemoteExec,
    /// `data "external"`.
    DataExternal,
    /// Une étiquette de `provisioner` ou de `data` qui n'est pas un littéral
    /// simple : elle ne peut pas être jugée sans évaluer le langage.
    EtiquetteNonLitterale(String),
    /// Un fichier `.tf.json` ou `.tofu.json`, que ce vérificateur ne lit pas.
    FichierJson,
    /// Pas de `.terraform.lock.hcl` à la racine du module.
    VerrouAbsent,
    /// Un fournisseur du verrou hors de la liste blanche.
    FournisseurHorsListe(String),
    /// Une entrée `provider` du verrou qui n'est pas un littéral simple.
    FournisseurIllisible(String),
    /// La liste blanche est vide : tout est refusé.
    ListeBlancheVide,
    /// Un bloc `module` dont la source n'est pas un chemin local.
    SourceDistante(String),
    /// Un bloc `module` dont la source locale sort de la racine du dépôt.
    SourceHorsRacine(String),
    /// Un bloc `module` dont la source locale ne vise pas
    /// `_shared/terraform/modules/<nom>/`.
    SourceHorsModulesPartages(String),
    /// Un bloc `module` sans source littérale.
    SourceIllisible,
}

impl fmt::Display for Construction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Construction::ProvisionerLocalExec => {
                f.write_str("provisioner \"local-exec\" interdit (commande locale arbitraire)")
            }
            Construction::ProvisionerRemoteExec => {
                f.write_str("provisioner \"remote-exec\" interdit (commande distante arbitraire)")
            }
            Construction::DataExternal => {
                f.write_str("data \"external\" interdit (commande locale arbitraire, dès le plan)")
            }
            Construction::EtiquetteNonLitterale(brut) => write!(
                f,
                "étiquette non littérale « {brut} » refusée : elle ne peut pas être jugée"
            ),
            Construction::FichierJson => {
                f.write_str("fichier JSON refusé : seule la syntaxe HCL est vérifiée")
            }
            Construction::VerrouAbsent => write!(
                f,
                "{FICHIER_VERROU} absent : les fournisseurs doivent être verrouillés"
            ),
            Construction::FournisseurHorsListe(nom) => write!(
                f,
                "fournisseur « {nom} » hors de la liste blanche [tofu] fournisseurs_autorises"
            ),
            Construction::FournisseurIllisible(brut) => {
                write!(f, "fournisseur non littéral « {brut} » refusé")
            }
            Construction::ListeBlancheVide => f.write_str(
                "liste blanche [tofu] fournisseurs_autorises vide : tout fournisseur est refusé",
            ),
            Construction::SourceDistante(source) => write!(
                f,
                "source de module distante « {source} » refusée : un module est copié \
                 dans le dépôt, sous _shared/terraform/modules/<nom>/"
            ),
            Construction::SourceHorsRacine(source) => write!(
                f,
                "source de module « {source} » refusée : elle sort de la racine du dépôt"
            ),
            Construction::SourceHorsModulesPartages(source) => write!(
                f,
                "source de module « {source} » refusée : elle ne vise pas \
                 _shared/terraform/modules/<nom>/"
            ),
            Construction::SourceIllisible => f.write_str(
                "bloc module sans source littérale : seule une chaîne simple est admise",
            ),
        }
    }
}

/// Une violation du contrat, située.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Fichier en cause, relatif à la racine du dépôt.
    pub fichier: String,
    /// Ligne, à partir de 1, quand la violation en a une.
    pub ligne: Option<usize>,
    /// Ce qui est refusé.
    pub construction: Construction,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ligne {
            Some(ligne) => write!(f, "{}:{} : {}", self.fichier, ligne, self.construction),
            None => write!(f, "{} : {}", self.fichier, self.construction),
        }
    }
}

/// Vérifie un module contre le contrat d'ADR-034, point 3.
///
/// Rend la liste des violations, vide si et seulement si le module peut être
/// lancé. L'ordre suit celui des fichiers, puis celui des lignes.
pub fn verifier_module(module: &ModuleSoumis, fournisseurs_autorises: &[String]) -> Vec<Violation> {
    let mut violations = Vec::new();
    let chemin_verrou = joindre(&module.chemin, FICHIER_VERROU);

    if fournisseurs_autorises.iter().all(|f| f.trim().is_empty()) {
        violations.push(Violation {
            fichier: "configuration".to_string(),
            ligne: None,
            construction: Construction::ListeBlancheVide,
        });
    }

    let mut verrou_vu = false;
    for fichier in &module.fichiers {
        let nom = fichier.chemin.rsplit('/').next().unwrap_or_default();
        if fichier.chemin == chemin_verrou {
            verrou_vu = true;
            verifier_verrou(fichier, fournisseurs_autorises, &mut violations);
        } else if EXTENSIONS_JSON_REFUSEES.iter().any(|e| nom.ends_with(e)) {
            violations.push(Violation {
                fichier: fichier.chemin.clone(),
                ligne: None,
                construction: Construction::FichierJson,
            });
        } else if EXTENSIONS_PROGRAMME.iter().any(|e| nom.ends_with(e)) {
            verifier_programme(fichier, &mut violations);
        }
    }

    if !verrou_vu {
        violations.push(Violation {
            fichier: chemin_verrou,
            ligne: None,
            construction: Construction::VerrouAbsent,
        });
    }
    violations
}

fn joindre(dossier: &str, nom: &str) -> String {
    let dossier = dossier.trim_matches('/');
    if dossier.is_empty() {
        nom.to_string()
    } else {
        format!("{dossier}/{nom}")
    }
}

fn verifier_verrou(fichier: &FichierModule, autorises: &[String], violations: &mut Vec<Violation>) {
    let texte = fichier.contenu.as_str();
    for debut in occurrences(texte, "provider") {
        let apres = sauter_blancs(texte, debut + "provider".len());
        let construction = match lire_etiquette(texte, apres) {
            Some((Etiquette::Litterale(nom), _)) => {
                if autorises.iter().any(|a| a.trim() == nom) {
                    continue;
                }
                Construction::FournisseurHorsListe(nom)
            }
            Some((Etiquette::NonLitterale(brut), _)) => Construction::FournisseurIllisible(brut),
            None => continue,
        };
        violations.push(Violation {
            fichier: fichier.chemin.clone(),
            ligne: Some(ligne_de(texte, debut)),
            construction,
        });
    }
}

fn verifier_programme(fichier: &FichierModule, violations: &mut Vec<Violation>) {
    let texte = fichier.contenu.as_str();
    let mut trouvees: Vec<(usize, Construction)> = Vec::new();

    for (mot, etiquette_interdite, construction) in INTERDITS {
        for debut in occurrences(texte, mot) {
            let apres = sauter_blancs(texte, debut + mot.len());
            match lire_etiquette(texte, apres) {
                Some((Etiquette::Litterale(etiquette), _)) if etiquette == *etiquette_interdite => {
                    trouvees.push((debut, construction.clone()));
                }
                Some((Etiquette::NonLitterale(brut), _)) => {
                    let violation = (debut, Construction::EtiquetteNonLitterale(brut));
                    if !trouvees.contains(&violation) {
                        trouvees.push(violation);
                    }
                }
                _ => {}
            }
        }
    }

    for debut in occurrences(texte, "module") {
        if let Some(construction) = juger_bloc_module(texte, debut, &fichier.chemin) {
            trouvees.push((debut, construction));
        }
    }

    trouvees.sort_by_key(|(debut, _)| *debut);
    violations.extend(trouvees.into_iter().map(|(debut, construction)| Violation {
        fichier: fichier.chemin.clone(),
        ligne: Some(ligne_de(texte, debut)),
        construction,
    }));
}

/// Juge un bloc `module "nom" { … }` commençant à `debut`. Rend `None` si ce
/// n'est pas un bloc (une référence `module.x`, par exemple) ou s'il est
/// conforme.
fn juger_bloc_module(texte: &str, debut: usize, fichier: &str) -> Option<Construction> {
    let apres_mot = sauter_blancs(texte, debut + "module".len());
    let (_, apres_etiquette) = lire_etiquette(texte, apres_mot)?;
    let ouverture = sauter_blancs(texte, apres_etiquette);
    if !texte[ouverture..].starts_with('{') {
        return None;
    }
    let fin = fin_de_bloc(texte, ouverture);
    let corps = &texte[ouverture + 1..fin];

    let Some(source) = lire_source(corps) else {
        return Some(Construction::SourceIllisible);
    };
    juger_source(&source, fichier)
}

/// Lit la valeur de l'attribut `source` d'un corps de bloc, si c'est une
/// chaîne littérale simple.
fn lire_source(corps: &str) -> Option<String> {
    for debut in occurrences(corps, "source") {
        let egal = sauter_blancs(corps, debut + "source".len());
        if !corps[egal..].starts_with('=') {
            continue;
        }
        let valeur = sauter_blancs(corps, egal + 1);
        if !corps[valeur..].starts_with('"') {
            return None;
        }
        return match lire_etiquette(corps, valeur) {
            Some((Etiquette::Litterale(source), _)) => Some(source),
            _ => None,
        };
    }
    None
}

fn juger_source(source: &str, fichier: &str) -> Option<Construction> {
    if !(source.starts_with("./") || source.starts_with("../")) {
        return Some(Construction::SourceDistante(source.to_string()));
    }
    // Le dossier du fichier qui appelle, puis la source, résolue sans jamais
    // remonter au-dessus de la racine du dépôt.
    let mut segments: Vec<&str> = fichier.split('/').collect();
    segments.pop();
    for segment in source.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Some(Construction::SourceHorsRacine(source.to_string()));
                }
            }
            autre => segments.push(autre),
        }
    }
    let sous_partages = segments.len() > MODULES_PARTAGES.len()
        && segments[..MODULES_PARTAGES.len()] == *MODULES_PARTAGES;
    if sous_partages {
        None
    } else {
        Some(Construction::SourceHorsModulesPartages(source.to_string()))
    }
}

// ── Lecture lexicale ─────────────────────────────────────────

/// Une étiquette de bloc, littérale ou non.
enum Etiquette {
    Litterale(String),
    NonLitterale(String),
}

fn est_car_identifiant(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// Positions où `mot` apparaît comme mot entier. Un `.` avant le mot l'exclut :
/// `var.module` n'ouvre rien.
fn occurrences<'a>(texte: &'a str, mot: &'a str) -> impl Iterator<Item = usize> + 'a {
    texte.match_indices(mot).filter_map(move |(debut, _)| {
        let avant = texte[..debut].chars().next_back();
        let apres = texte[debut + mot.len()..].chars().next();
        let borne_avant = avant.is_none_or(|c| !est_car_identifiant(c) && c != '.');
        let borne_apres = apres.is_none_or(|c| !est_car_identifiant(c));
        (borne_avant && borne_apres).then_some(debut)
    })
}

/// Saute les blancs, retours à la ligne et commentaires (`#`, `//`, `/* */`).
fn sauter_blancs(texte: &str, mut i: usize) -> usize {
    loop {
        let reste = &texte[i..];
        let Some(c) = reste.chars().next() else {
            return i;
        };
        if c.is_whitespace() {
            i += c.len_utf8();
        } else if reste.starts_with('#') || reste.starts_with("//") {
            i += reste.find('\n').unwrap_or(reste.len());
        } else if let Some(commentaire) = reste.strip_prefix("/*") {
            i += commentaire.find("*/").map_or(reste.len(), |fin| fin + 4);
        } else {
            return i;
        }
    }
}

/// Lit une étiquette : chaîne entre guillemets ou identifiant nu, les deux
/// formes qu'admet HCL.
fn lire_etiquette(texte: &str, i: usize) -> Option<(Etiquette, usize)> {
    let reste = &texte[i..];
    let premier = reste.chars().next()?;
    if premier == '"' {
        let mut fin = None;
        let mut echappe = false;
        for (position, c) in reste[1..].char_indices() {
            if c == '\n' {
                return None;
            }
            if echappe {
                echappe = false;
            } else if c == '\\' {
                echappe = true;
            } else if c == '"' {
                fin = Some(position);
                break;
            }
        }
        let fin = fin?;
        let brut = &reste[1..1 + fin];
        let suivant = i + 1 + fin + 1;
        if brut.contains('\\') || brut.contains("${") || brut.contains("%{") {
            Some((Etiquette::NonLitterale(brut.to_string()), suivant))
        } else {
            Some((Etiquette::Litterale(brut.to_string()), suivant))
        }
    } else if est_car_identifiant(premier) {
        let longueur = reste
            .find(|c: char| !est_car_identifiant(c))
            .unwrap_or(reste.len());
        Some((
            Etiquette::Litterale(reste[..longueur].to_string()),
            i + longueur,
        ))
    } else {
        None
    }
}

/// Position de l'accolade fermante du bloc ouvert en `ouverture`, en ignorant
/// les accolades des chaînes et des commentaires. Fin du texte à défaut.
fn fin_de_bloc(texte: &str, ouverture: usize) -> usize {
    let mut profondeur = 0usize;
    let mut i = ouverture;
    while i < texte.len() {
        let reste = &texte[i..];
        let Some(c) = reste.chars().next() else {
            break;
        };
        if reste.starts_with('#') || reste.starts_with("//") || reste.starts_with("/*") {
            i = sauter_blancs(texte, i);
            continue;
        }
        match c {
            '"' => {
                i = match lire_etiquette(texte, i) {
                    Some((_, suivant)) => suivant,
                    None => i + 1,
                };
                continue;
            }
            '{' => profondeur += 1,
            '}' => {
                profondeur = profondeur.saturating_sub(1);
                if profondeur == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += c.len_utf8();
    }
    texte.len()
}

fn ligne_de(texte: &str, position: usize) -> usize {
    texte[..position].matches('\n').count() + 1
}
