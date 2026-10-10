import { Hero } from "../components/Hero";
import { Playground } from "../components/Playground";
import { Benchmarks, Code, Cta, Features, Manifesto, Marquee, Steps } from "../components/Sections";
import { t } from "../i18n";
import type { Lang } from "../routes";
import { GITHUB, ORIGIN, Seo } from "../seo";
import { alternates } from "./seo-helpers";

function Curtain() {
  // Pure CSS: shown only when the inline boot script adds `html.intro` (first visit, motion allowed).
  return (
    <div className="curtain" aria-hidden="true">
      <svg viewBox="0 0 120 120">
        <circle className="ring" cx="60" cy="60" r="52" pathLength="100" />
        <circle className="dot" cx="96.8" cy="23.2" r="7" />
      </svg>
      <span className="count" />
      <span className="word">active</span>
    </div>
  );
}

export function Home({ lang }: { lang: Lang }) {
  const d = t(lang);
  return (
    <>
      <Seo
        title={d.home.title}
        titleTemplate="%s"
        description={d.home.description}
        canonical={`/${lang}/`}
        alternates={alternates({ lang, page: "home" })}
        locale={d.locale}
        jsonLd={[
          {
            "@context": "https://schema.org",
            "@type": "SoftwareSourceCode",
            name: "active-react",
            description: d.home.description,
            codeRepository: GITHUB,
            programmingLanguage: ["TypeScript", "Rust"],
            license: "https://opensource.org/licenses/MIT",
            url: `${ORIGIN}/${lang}/`,
          },
          { "@context": "https://schema.org", "@type": "WebSite", name: "active", url: `${ORIGIN}/`, inLanguage: lang },
        ]}
      />
      <Curtain />
      <Hero lang={lang} />
      <Marquee lang={lang} />
      <Manifesto lang={lang} />
      <Features lang={lang} />
      <Code lang={lang} />
      <Playground lang={lang} />
      <Benchmarks lang={lang} />
      <Steps lang={lang} />
      <Cta lang={lang} />
    </>
  );
}
