# ADR-011 : Gate de scan différencié par image

- **Statut** : contresigné le 2026-10-18.
- **Sorti de l'index le 2026-10-21** (revue des ADR, décision 7) : texte déplacé depuis `README.md` sans modification de fond.
- **Amendé le 2026-10-28** (sluis#8) : le binaire `k6` rejoint `tofu` dans l'exception du gate bloquant. Amendement proposé, contresigné le 2026-10-28.

**Contexte, mesuré le 2026-09-02.** Le gate du plancher scanne chaque image en
`CRITICAL,HIGH` avec `ignore-unfixed`, et refuse de pousser si quoi que ce soit
sort. L'image exécutante en fait sortir **12**, toutes dans le binaire `tofu`,
c'est-à-dire dans le `go.mod` d'OpenTofu : `x/crypto`, `x/net`, `x/mod`,
`x/text`, `grpc`, `oras`. Aucune dans les paquets système, aucune dans wrk.

**Ce n'est pas un problème de moteur.** Mesuré à charge égale le même jour :
Terraform v1.16.1 en porte **9** dans son propre binaire, plus 2 dans sa base
Alpine. Changer de moteur déplace le chiffre, pas la nature du problème, qui est
d'embarquer un binaire Go tiers dans une image que l'on scanne.

**Décision.** Le gate devient **différencié, par image et non par sévérité** :

- **Sluis public** — gate plein, `CRITICAL,HIGH` bloquant sur tout. C'est
  l'image exposée à Internet, et tout ce qu'elle porte est de notre fait.
- **Sluis exécutant** — bloquant sur les paquets système et sur wrk, qui sont de
  notre ressort ; le binaire `tofu` est écarté du gate bloquant, et **de lui
  seul**, par `skip-files`.

Deux garde-fous, sans quoi la décision serait un simple relâchement :

1. **Ce qui est écarté est rapporté.** Un second scan, non bloquant, rejoue le
   scan complet et laisse les CVE de `tofu` lisibles dans le journal de chaque
   build. Une exception muette redevient un angle mort à la première montée de
   version.
2. **La version d'OpenTofu est épinglée dans le Dockerfile et se remonte.** La
   1.8.8 en portait 53, la 1.12.6 en porte 12 : le chiffre est un indicateur à
   suivre, pas une fatalité. *Relevé du 2026-10-29* : la 1.12.6, non remontée,
   était passée à 22 ; la 1.13.2 en porte 5.

**Alternatives écartées.** Un `.trivyignore` daté listant les 12 CVE : il attrape
mieux les nouveautés, mais il est à refaire à chaque montée de version et pourrit
dès que personne ne le relit. Laisser le gate rouge et ne pas publier l'exécutant :
défendable aujourd'hui puisque H1 n'est pas levée, mais un job rouge en permanence
finit par ne plus être lu, et c'est alors tout le plancher qui est perdu.

**Ce que la décision ne couvre pas, et qu'il faut savoir.** Les bibliothèques
qu'`wrk` réclame sont copiées dans l'image comme des fichiers, sans leur
enregistrement `dpkg`. Trivy ne les rattache donc à aucun paquet et ne les
signale pas. Elles viennent de la même Debian 12 que la base et se remettent à
jour à chaque reconstruction, ce qui limite la portée du trou sans l'annuler.

Cet ADR n'est pas irréversible : rétablir le gate plein est une ligne de
configuration. Il a été contresigné le 2026-10-18, parce qu'il relâche un garde-fou du
plancher.

**Amendement du 2026-10-28 : k6 rejoint l'exception** (sluis#8, ADR-010).
Quand `wrk` est remplacé par k6, le binaire `k6` est traité **exactement comme
`tofu`** : binaire Go tiers dont le `go.mod` portera vraisemblablement des CVE
hors de notre ressort, il est écarté du gate bloquant par `skip-files`, et de
lui seul. Les deux garde-fous s'appliquent tels quels : le scan complet non
bloquant laisse ses CVE lisibles dans chaque build, et sa version est épinglée
dans le Dockerfile et se remonte.

Le gate bloquant de l'image exécutante porte alors sur les paquets système, et
sur eux seuls. **Le trou décrit plus haut disparaît** : il n'y a plus de
bibliothèques copiées sans enregistrement `dpkg`, puisque k6 est un binaire
statique. Jusqu'au remplacement, la décision d'origine reste en vigueur.
Proposé, contresigné le 2026-10-28.
