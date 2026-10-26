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

/// Longer official names and alternative spellings, plus common ISO alpha-3 codes.
const ALIASES: &str = "Korea Republic Of:KR;Republic of Korea:KR;Iran Islamic Republic Of:IR;Taiwan Province Of China:TW;Chinese Taipei:TW;\
Hong Kong SAR:HK;Hong Kong SAR China:HK;Macao SAR China:MO;The Former Yugoslav Republic of Macedonia:MK;Bolivia Plurinational State Of:BO;\
Venezuela Bolivarian Republic Of:VE;Moldova Republic Of:MD;Tanzania:TZ;Tanzania United Republic Of:TZ;Syria:SY;Syrian Arab Republic:SY;\
Lao People's Democratic Republic:LA;Laos:LA;Brunei Darussalam:BN;Holland:NL;The Netherlands:NL;Netherlands The:NL;Czech:CZ;\
Slovak Republic:SK;Britain:GB;U.K.:GB;U.S.:US;U.S.A.:US;America:US;Deutschland:DE;Espana:ES;Italia:IT;Brasil:BR;Suisse:CH;Schweiz:CH;\
Osterreich:AT;Belgique:BE;Belgie:BE;Polska:PL;Ivory Coast:CI;Cote d'Ivoire:CI;Cape Verde:CV;Cabo Verde:CV;Myanmar:MM;Burma:MM;\
Lebanon:LB;Uganda:UG;Senegal:SN;Rwanda:RW;Madagascar:MG;Malawi:MW;Mali:ML;Haiti:HT;Guyana:GY;Suriname:SR;Belize:BZ;Aruba:AW;\
Curacao:CW;Bermuda:BM;Monserrat:MS;Andorra:AD;San Marino:SM;Vatican:VA;Albania:AL;Armenia:AM;Azerbaijan:AZ;Kyrgyz Republic:KG;\
Tajikistan:TJ;Turkmenistan:TM;Afghanistan:AF;Bhutan:BT;Maldives:MV;New Caledonia:NC;French Polynesia:PF;Papua New Guinea:PG;Samoa:WS;\
Tonga:TO;Reunion:RE;Martinique:MQ;Guadeloupe:GP;Greenland:GL;Faroe:FO;Aland Islands:AX;Palestine:PS;Yemen:YE;Libya:LY;Sudan:SD;\
Algeria:DZ;Botswana:BW;Zambia:ZM;Angola:AO;Cameroon:CM;Ethiopia:ET;Ghana:GH;Kenya:KE;Nigeria:NG;Mozambique:MZ;Namibia:NA;Zimbabwe:ZW;\
USA:US;GBR:GB;DEU:DE;GER:DE;FRA:FR;ITA:IT;ESP:ES;NLD:NL;NED:NL;BEL:BE;CHE:CH;SUI:CH;AUT:AT;POL:PL;CZE:CZ;SVK:SK;HUN:HU;ROU:RO;BGR:BG;\
GRC:GR;PRT:PT;POR:PT;IRL:IE;DNK:DK;DEN:DK;SWE:SE;NOR:NO;FIN:FI;ISL:IS;EST:EE;LVA:LV;LTU:LT;UKR:UA;RUS:RU;TUR:TR;ISR:IL;ARE:AE;SAU:SA;\
QAT:QA;IND:IN;CHN:CN;JPN:JP;KOR:KR;TWN:TW;HKG:HK;SGP:SG;MYS:MY;THA:TH;IDN:ID;PHL:PH;VNM:VN;AUS:AU;NZL:NZ;CAN:CA;MEX:MX;BRA:BR;ARG:AR;\
CHL:CL;COL:CO;PER:PE;URY:UY;ZAF:ZA;EGY:EG;MAR:MA;HRV:HR;SVN:SI;SRB:RS;BIH:BA;LUX:LU;MLT:MT;CYP:CY";

/// Lower case, accents removed, punctuation as single spaces, without "the" / "flag of".
fn norm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.trim().chars() {
        let c = match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' | 'Ø' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            '\'' | '’' => continue,
            c if c.is_alphanumeric() => c.to_ascii_lowercase(),
            _ => ' ',
        };
        if c == ' ' && (out.is_empty() || out.ends_with(' ')) {
            continue;
        }
        out.push(c);
    }
    let out = out.trim_end().to_string();
    let out = out.strip_prefix("flag of ").unwrap_or(&out).to_string();
    out.strip_prefix("the ").unwrap_or(&out).to_string()
}

fn entries() -> impl Iterator<Item = (&'static str, &'static str)> {
    COUNTRIES.split(';').chain(ALIASES.split(';')).filter_map(|e| e.trim().rsplit_once(':'))
}

/// iRacing flair IDs seen in real Data API responses (flair_id → ISO code).
const FLAIR_IDS: &[(i64, &str)] =
    &[(13, "AR"), (16, "AU"), (31, "BR"), (39, "CA"), (71, "FR"), (77, "DE"), (79, "GI"), (104, "JP"), (168, "PT"), (222, "GB"), (223, "US")];

/// Best country for a driver from every flair field iRacing may send: an ISO country code,
/// a three-letter short name, the flair name, or a known flair ID.
pub fn driver_country(iso: &str, short_name: &str, name: &str, flair_id: i64) -> &'static str {
    for candidate in [iso, short_name, name] {
        let c = country_code(candidate);
        if !c.is_empty() {
            return c;
        }
    }
    FLAIR_IDS.iter().find(|(id, _)| *id == flair_id).map(|(_, c)| *c).unwrap_or("")
}

/// ISO country code (e.g. "GB", or "GB-SCT" for UK nations) for the flair a driver picked on
/// their iRacing account. Returns "" when the flair is not a country, so no wrong flag is shown.
pub fn country_code(flair: &str) -> &'static str {
    let f = norm(flair);
    if f.is_empty() || f == "none" {
        return "";
    }
    // Exact name, alias or ISO code.
    if let Some((_, c)) = entries().find(|(n, c)| norm(n) == f || (f.len() == 2 && c.eq_ignore_ascii_case(&f))) {
        return c;
    }
    // Longest country name contained in the flair (e.g. "Flag - Germany", "Germany (DE)").
    let padded = format!(" {f} ");
    entries()
        .filter(|(n, _)| n.len() >= 4)
        .filter(|(n, _)| padded.contains(&format!(" {} ", norm(n))))
        .max_by_key(|(n, _)| n.len())
        .map(|(_, c)| c)
        .unwrap_or("")
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
        assert_eq!(country_code("United Kingdom"), "GB");
        assert_eq!(country_code("germany"), "DE");
        assert_eq!(country_code("NL"), "NL");
        assert_eq!(country_code("GER"), "DE");
        assert_eq!(country_code("Türkiye"), "TR");
        assert_eq!(country_code("Korea, Republic Of"), "KR");
        assert_eq!(country_code("The Netherlands"), "NL");
        assert_eq!(country_code("United States of America"), "US");
        assert_eq!(country_code("Flag of Scotland"), "GB-SCT");
        assert_eq!(country_code("Brazil (BR)"), "BR");
        assert_eq!(country_code("Côte d'Ivoire"), "CI");
        assert_eq!(country_code("None"), "");
        assert_eq!(country_code("iRacing"), "");
        assert_eq!(driver_country("", "", "", 223), "US");
        assert_eq!(driver_country("AU", "AUS", "Australia", 16), "AU");
        assert_eq!(driver_country("", "DEU", "", 0), "DE");
        assert_eq!(driver_country("", "", "", 0), "");
    }

    #[test]
    fn makes() {
        assert_eq!(make("Porsche 911 GT3 R (992)", "porsche992rgt3"), "Porsche");
        assert_eq!(make("Mercedes-AMG GT3 2020", ""), "Mercedes-AMG");
        assert_eq!(make("Global Mazda MX-5 Cup", ""), "Mazda");
        assert_eq!(make("Hypercar 963", ""), "Hypercar");
    }
}
