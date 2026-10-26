//! Static lookups: country names to ISO codes (for flags) and car names to manufacturers.

const COUNTRIES: &str = "Afghanistan:AF;Albania:AL;Algeria:DZ;Andorra:AD;Angola:AO;Argentina:AR;Armenia:AM;Australia:AU;Austria:AT;\
Azerbaijan:AZ;Bahamas:BS;Bahrain:BH;Bangladesh:BD;Barbados:BB;Belarus:BY;Belgium:BE;Belize:BZ;Bermuda:BM;Bolivia:BO;\
Bosnia and Herzegovina:BA;Botswana:BW;Brazil:BR;Brunei:BN;Bulgaria:BG;Cambodia:KH;Cameroon:CM;Canada:CA;Cayman Islands:KY;Chile:CL;\
China:CN;Colombia:CO;Costa Rica:CR;Croatia:HR;Cuba:CU;Cyprus:CY;Czech Republic:CZ;Czechia:CZ;Denmark:DK;Dominican Republic:DO;\
Ecuador:EC;Egypt:EG;El Salvador:SV;England:GB-ENG;Estonia:EE;Ethiopia:ET;Faroe Islands:FO;Fiji:FJ;Finland:FI;France:FR;Georgia:GE;\
Germany:DE;Ghana:GH;Gibraltar:GI;Greece:GR;Greenland:GL;Guatemala:GT;Guernsey:GG;Honduras:HN;Hong Kong:HK;Hungary:HU;Iceland:IS;\
India:IN;Indonesia:ID;Iran:IR;Iraq:IQ;Ireland:IE;Isle of Man:IM;Israel:IL;Italy:IT;Jamaica:JM;Japan:JP;Jersey:JE;Jordan:JO;\
Kazakhstan:KZ;Kenya:KE;Korea:KR;South Korea:KR;Korea, Republic of:KR;Kosovo:XK;Kuwait:KW;Kyrgyzstan:KG;Latvia:LV;Lebanon:LB;\
Liechtenstein:LI;Lithuania:LT;Luxembourg:LU;Macau:MO;Macao:MO;Malaysia:MY;Malta:MT;Mauritius:MU;Mexico:MX;Moldova:MD;Monaco:MC;\
Mongolia:MN;Montenegro:ME;Morocco:MA;Mozambique:MZ;Namibia:NA;Nepal:NP;Netherlands:NL;New Zealand:NZ;Nicaragua:NI;Nigeria:NG;\
North Macedonia:MK;Macedonia:MK;Northern Ireland:GB-NIR;Norway:NO;Oman:OM;Pakistan:PK;Panama:PA;Paraguay:PY;Peru:PE;\
Philippines:PH;Poland:PL;Portugal:PT;Puerto Rico:PR;Qatar:QA;Romania:RO;Russia:RU;Russian Federation:RU;Saudi Arabia:SA;\
Scotland:GB-SCT;Serbia:RS;Singapore:SG;Slovakia:SK;Slovenia:SI;South Africa:ZA;Spain:ES;Sri Lanka:LK;Sweden:SE;Switzerland:CH;\
Taiwan:TW;Thailand:TH;Trinidad and Tobago:TT;Tunisia:TN;Turkey:TR;Türkiye:TR;Ukraine:UA;United Arab Emirates:AE;UAE:AE;\
United Kingdom:GB;Great Britain:GB;UK:GB;United States:US;United States of America:US;USA:US;Uruguay:UY;Uzbekistan:UZ;\
Venezuela:VE;Vietnam:VN;Viet Nam:VN;Wales:GB-WLS;Zambia:ZM;Zimbabwe:ZW";

/// Legacy iRacing clubs that cover exactly one country.
const CLUBS: &str = "Atlantic:US;California:US;Carolina:US;Florida:US;Georgia:US;Mid-South:US;Midwest:US;New England:US;New York:US;\
Northwest:US;Ohio:US;Pennsylvania:US;Plains:US;Texas:US;Virginias:US;West:US;Michigan:US;Illinois:US;Indiana:US;Southwest:US;\
UK and I:GB;Brazil:BR;Italy:IT;Japan:JP;France:FR;Finland:FI;Canada:CA;Argentina:AR;Mexico:MX;Australia/NZ:AU;Australia & NZ:AU";

fn find(table: &'static str, name: &str) -> Option<&'static str> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    table.split(';').find_map(|e| {
        let (n, c) = e.trim().rsplit_once(':')?;
        n.eq_ignore_ascii_case(name).then_some(c)
    })
}

/// ISO country code for an iRacing flair name, falling back to a single-country club.
pub fn country_code(flair: &str, club: &str) -> &'static str {
    let f = flair.trim();
    // A flair may already be an ISO code.
    if f.len() == 2 && f.chars().all(|c| c.is_ascii_alphabetic()) {
        return COUNTRIES.split(';').find_map(|e| e.trim().rsplit_once(':').map(|(_, c)| c).filter(|c| c.eq_ignore_ascii_case(f))).unwrap_or("");
    }
    find(COUNTRIES, f).or_else(|| find(CLUBS, club)).unwrap_or("")
}

const MAKES: &[(&str, &str)] = &[
    ("acura", "Acura"),
    ("alpine", "Alpine"),
    ("aston", "Aston Martin"),
    ("audi", "Audi"),
    ("bmw", "BMW"),
    ("cadillac", "Cadillac"),
    ("camaro", "Chevrolet"),
    ("chevrolet", "Chevrolet"),
    ("chevy", "Chevrolet"),
    ("corvette", "Chevrolet"),
    ("dallara", "Dallara"),
    ("dodge", "Dodge"),
    ("ferrari", "Ferrari"),
    ("ford", "Ford"),
    ("mustang", "Ford"),
    ("formula vee", "Volkswagen"),
    ("genesis", "Genesis"),
    ("holden", "Holden"),
    ("honda", "Honda"),
    ("hyundai", "Hyundai"),
    ("kia", "Kia"),
    ("lamborghini", "Lamborghini"),
    ("ligier", "Ligier"),
    ("lotus", "Lotus"),
    ("mazda", "Mazda"),
    ("mx-5", "Mazda"),
    ("mclaren", "McLaren"),
    ("mercedes", "Mercedes-AMG"),
    ("amg", "Mercedes-AMG"),
    ("mini", "Mini"),
    ("nissan", "Nissan"),
    ("oreca", "Oreca"),
    ("peugeot", "Peugeot"),
    ("pontiac", "Pontiac"),
    ("porsche", "Porsche"),
    ("radical", "Radical"),
    ("renault", "Renault"),
    ("riley", "Riley"),
    ("skip barber", "Skip Barber"),
    ("subaru", "Subaru"),
    ("super formula", "Super Formula"),
    ("toyota", "Toyota"),
    ("gr86", "Toyota"),
    ("supra", "Toyota"),
    ("volkswagen", "Volkswagen"),
    ("vw", "Volkswagen"),
    ("williams", "Williams"),
];

/// Manufacturer of a car from its screen name or path; falls back to the first word.
pub fn make(name: &str, path: &str) -> String {
    let hay = format!("{} {}", name.to_lowercase(), path.to_lowercase().replace(['_', '/'], " "));
    if let Some((_, m)) = MAKES.iter().find(|(k, _)| hay.contains(k)) {
        return (*m).to_string();
    }
    name.split_whitespace().next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countries() {
        assert_eq!(country_code("United Kingdom", ""), "GB");
        assert_eq!(country_code("germany", ""), "DE");
        assert_eq!(country_code("", "California"), "US");
        assert_eq!(country_code("", "DE-AT-CH"), "");
        assert_eq!(country_code("NL", ""), "NL");
    }

    #[test]
    fn makes() {
        assert_eq!(make("Porsche 911 GT3 R (992)", "porsche992rgt3"), "Porsche");
        assert_eq!(make("Mercedes-AMG GT3 2020", ""), "Mercedes-AMG");
        assert_eq!(make("Global Mazda MX-5 Cup", ""), "Mazda");
        assert_eq!(make("Hypercar 963", ""), "Hypercar");
    }
}
