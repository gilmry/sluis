# Procédure d'incident : <nom du SaaS>

Modèle F1 (sluis#112). Le SaaS est **sous-traitant** : il prévient le client,
responsable du traitement, qui décide seul de notifier l'autorité de contrôle
et les personnes (RGPD, articles 33.2 et 28.3.f).

**Engagement contractuel** : prévenir le client **sous 48 heures après avoir
pris connaissance** de la violation, puis transmettre les informations au fur et
à mesure, sans attendre d'avoir tout.

## 1. Qualifier (dans l'heure)

1. Noter l'heure exacte de **prise de connaissance**. C'est elle qui fait
   courir les 48 heures.
2. Répondre à trois questions, par oui, non ou inconnu :
   - des données personnelles sont-elles touchées ?
   - leur confidentialité, leur intégrité ou leur disponibilité est-elle
     atteinte ?
   - l'atteinte est-elle en cours ?
3. Une réponse « oui » ou « inconnu » à la première question ouvre un incident
   de violation de données. Le doute suffit : on qualifie plus tard, on
   prévient d'abord.
4. Ouvrir la fiche de l'incident (section 5) dans le dépôt privé du SaaS.

## 2. Geler les preuves (avant toute correction)

Ne rien supprimer, ne rien redémarrer avant ces trois gestes, sauf si l'atteinte
est en cours et que la couper l'exige. Dans ce cas, noter ce qui a été coupé et
à quelle heure.

1. **Instantané** du serveur et des volumes chez l'hébergeur.
2. **Copie des journaux** de la période, depuis le stockage distant en ajout
   seul (épique B), pas depuis le serveur touché.
3. **Empreintes** (SHA-256) de chaque fichier copié, écrites dans la fiche.

## 3. Contenir et corriger

1. Couper l'accès en cause : clé, compte, jeton, port.
2. Corriger, puis vérifier par le contrôle automatique qui aurait dû voir
   l'incident. S'il n'existe pas, il devient une story.
3. Toute rotation de clé ou suppression de données reste une décision du
   superviseur.

## 4. Prévenir le client

- **Sous 48 heures** après la prise de connaissance, avec le modèle de la
  section 6, même incomplet.
- Puis un complément à chaque élément nouveau, et au plus tard à la clôture.
- Canal : celui fixé au contrat, et un appel au contact désigné.

## 5. Fiche de l'incident

| Champ | Valeur |
|---|---|
| Identifiant | `<AAAA-MM-JJ-n>` |
| Prise de connaissance | `<date et heure, fuseau>` |
| Par qui, comment | `<alerte, personne, client>` |
| Client(s) concerné(s) | `<nom>` |
| Nature de l'atteinte | confidentialité / intégrité / disponibilité |
| Catégories de personnes | `<résidents, personnel, …>` |
| Nombre approximatif de personnes | `<n ou fourchette, ou inconnu>` |
| Catégories de données | `<identité, santé, …>` |
| Nombre approximatif d'enregistrements | `<n ou inconnu>` |
| Preuves gelées | `<instantané, copies, empreintes>` |
| Mesures prises | `<heure : action>` |
| Client prévenu le | `<date et heure>` |
| Compléments envoyés | `<dates>` |
| Clôture | `<date, cause, story ouverte>` |

## 6. Modèle de courrier au client

> **Objet : violation de données personnelles, <nom du SaaS>, incident <identifiant>**
>
> Madame, Monsieur,
>
> En notre qualité de sous-traitant, nous vous informons d'une violation de
> données personnelles dont nous avons pris connaissance le <date> à <heure>.
>
> **Ce que nous savons à ce stade :**
> - nature de la violation : <description simple> ;
> - catégories et nombre approximatif de personnes concernées : <…> ;
> - catégories et nombre approximatif d'enregistrements concernés : <…> ;
> - conséquences probables : <…> ;
> - mesures prises ou proposées pour y remédier et en limiter les effets : <…>.
>
> **Ce que nous ne savons pas encore :** <…>. Nous vous transmettrons ces
> éléments dès qu'ils seront connus.
>
> En tant que responsable du traitement, il vous revient de décider de la
> notification à l'autorité de contrôle dans les 72 heures suivant votre prise
> de connaissance, et de l'information des personnes concernées. Nous restons à
> votre disposition pour vous y aider.
>
> Contact pour cet incident : <nom, téléphone, courriel>.
>
> <signature>

## 7. Après la clôture

- Archiver la fiche, les preuves et les courriers, hors du serveur.
- Chaque cause devient une story, avec un test qui aurait vu l'incident.
- Rejouer l'exercice sur table (story F2) si la procédure a manqué.
