//! Huller i historikken. Cloudflare gemmer ca. en måned, så det, HAM ikke når at hente, er væk for altid. Hullerne udledes af `hentet` (ingen ny tabel): en dag uden
//! række er ikke hentet, og `hentet_utc` viser, hvornår programmet sidst arbejdede. Et døgn, der hentes igen efter et hul, får det nye tidspunkt, så et spring mellem to
//! tidspunkter er en periode, hvor intet blev hentet. `fra` kan ligge op til et døgn før programmet reelt stoppede: i dag og i går hentes igen og får det nye tidspunkt.
use std::collections::BTreeSet;

use serde::Serialize;

use crate::cf::Fejl;
use crate::store::Store;
use crate::vaern::{dag_ms, dag_noegle, parse_iso, DOEGN_MS, HENT_DAGE};

/// Et hul er over 2 døgn: så mange kalenderdage skal der mindst være mellem to hentninger.
pub const HUL_DAGE: i64 = 3;
/// Foran hullets nøgle i `indstillinger`: linjen er lukket.
const LUKKET: &str = "hul_lukket:";

/// En periode, hvor intet blev hentet. `tabt`: dage i perioden uden tal, som Cloudflare ikke længere har.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Hul {
    pub noegle: String,
    pub fra: String,
    pub til: String,
    pub tabt: i64,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
pub struct HullerUd {
    /// Hullerne, nyeste først. Fra `laes`: kun dem, hvor linjen ikke er lukket.
    pub linjer: Vec<Hul>,
    /// Alle dage uden tal fra første dag med tal til og med i går, ældste først.
    pub dage: Vec<String>,
    pub aeldste: Option<String>,
}

/// `hentet`: (dag, hentet_utc) for alle domæner. Ren funktion.
pub fn regn(hentet: &[(String, String)], nu: i64) -> HullerUd {
    let dage: BTreeSet<&str> = hentet.iter().map(|x| x.0.as_str()).collect();
    let Some(aeldste) = dage.first().map(|d| d.to_string()) else {
        return HullerUd::default();
    };
    let (start, i_gaar) = (dag_ms(&aeldste).unwrap_or(nu), nu - DOEGN_MS);
    let mangler: Vec<String> = (0..=(i_gaar - start).div_euclid(DOEGN_MS))
        .map(|i| dag_noegle(start + i * DOEGN_MS))
        .filter(|d| !dage.contains(d.as_str()))
        .collect();
    let graense = dag_noegle(nu - HENT_DAGE * DOEGN_MS);
    let kl: BTreeSet<i64> = hentet
        .iter()
        .filter_map(|x| parse_iso(&x.1))
        .map(|ms| ms.div_euclid(DOEGN_MS))
        .collect();
    let kl: Vec<i64> = kl.into_iter().collect();
    let mut linjer: Vec<Hul> = kl
        .windows(2)
        .filter(|p| p[1] - p[0] >= HUL_DAGE)
        .map(|p| {
            let (fra, til) = (dag_noegle(p[0] * DOEGN_MS), dag_noegle(p[1] * DOEGN_MS));
            let tabt = mangler.iter().filter(|d| **d >= fra && **d <= til && **d < graense).count() as i64;
            Hul {
                noegle: format!("{fra}_{til}"),
                fra,
                til,
                tabt,
            }
        })
        .collect();
    linjer.reverse();
    HullerUd {
        linjer,
        dage: mangler,
        aeldste: Some(aeldste),
    }
}

pub fn laes(lager: &Store, nu: i64) -> Result<HullerUd, Fejl> {
    let hentet = lager.alle_hentet()?;
    let mut ud = regn(&hentet, nu);
    let mut aabne = Vec::new();
    for h in ud.linjer {
        if lager.indstilling(&format!("{LUKKET}{}", h.noegle))?.is_none() {
            aabne.push(h);
        }
    }
    ud.linjer = aabne;
    Ok(ud)
}

/// Nøglen er to datoer: andet kommer ikke fra programmet.
pub fn luk(lager: &mut Store, noegle: &str) -> Result<(), Fejl> {
    let gyldig = noegle
        .split_once('_')
        .is_some_and(|(a, b)| dag_ms(a).is_some() && dag_ms(b).is_some());
    if !gyldig {
        return Err(Fejl::UgyldigDato);
    }
    lager.gem_indstilling(&format!("{LUKKET}{noegle}"), "1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaern::iso;

    /// 2026-11-03T08:08:00Z
    const NU: i64 = 1_793_693_280_000;

    fn d(i: i64) -> String {
        dag_noegle(NU - i * DOEGN_MS)
    }

    /// Dag i-dage-siden, hentet `t` dage siden.
    fn h(i: i64, t: i64) -> (String, String) {
        (d(i), iso(NU - t * DOEGN_MS))
    }

    #[test]
    fn ingen_hentninger_ingen_huller() {
        assert_eq!(regn(&[], NU), HullerUd::default());
    }

    #[test]
    fn et_hul_over_2_doegn_med_dage_cloudflare_ikke_har_mere() {
        // Programmet kørte d-60…d-41 og igen i dag, hvor de sidste 30 døgn blev hentet: d-40…d-31 er væk.
        let mut rk: Vec<_> = (41..=60).map(|i| h(i, i - 1)).collect();
        rk.extend((0..=30).map(|i| h(i, 0)));
        let u = regn(&rk, NU);
        assert_eq!(u.aeldste, Some(d(60)));
        assert_eq!(u.dage.len(), 10);
        assert_eq!((u.dage[0].as_str(), u.dage[9].as_str()), (d(40).as_str(), d(31).as_str()));
        assert_eq!(
            u.linjer,
            [Hul {
                noegle: format!("{}_{}", d(40), d(0)),
                fra: d(40),
                til: d(0),
                tabt: 10
            }]
        );
    }

    #[test]
    fn to_doegn_er_intet_hul_men_tre_er() {
        let rk = |pause: i64| vec![h(10, 10), h(10 - pause, 10 - pause)];
        assert!(regn(&rk(2), NU).linjer.is_empty());
        assert_eq!(regn(&rk(3), NU).linjer.len(), 1);
    }

    #[test]
    fn alt_hentet_igen_taber_intet() {
        let mut rk = vec![h(20, 19)];
        rk.extend((0..=19).map(|i| h(i, 0)));
        let u = regn(&rk, NU);
        assert!(u.dage.is_empty());
        assert_eq!(u.linjer[0].tabt, 0, "Cloudflare havde stadig tallene");
    }

    #[test]
    fn en_dag_koeen_stadig_kan_hente_er_ikke_tabt() {
        let rk = vec![h(10, 9), h(1, 0)];
        let u = regn(&rk, NU);
        assert_eq!(u.dage.len(), 8, "et hul, så længe den mangler");
        assert_eq!(u.linjer[0].tabt, 0, "men ikke tabt: den er inden for de 30 døgn");
    }

    #[test]
    fn lukkede_huller_huskes_pr_hul() {
        let mut l = Store::i_hukommelse().unwrap();
        for (dag, kl) in [h(40, 40), h(20, 20), h(1, 1)] {
            l.gem_doegn("z", &dag, &[], &kl).unwrap();
        }
        let u = laes(&l, NU).unwrap();
        assert_eq!(u.linjer.len(), 2);
        assert_eq!(u.linjer[0].fra, d(20), "nyeste først");
        luk(&mut l, &u.linjer[0].noegle).unwrap();
        let u = laes(&l, NU).unwrap();
        assert_eq!(u.linjer.iter().map(|x| x.fra.clone()).collect::<Vec<_>>(), [d(40)]);
        assert!(luk(&mut l, "x_y").is_err());
    }
}
