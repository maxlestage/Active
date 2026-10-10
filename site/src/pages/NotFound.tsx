import { Split } from "../components/Split";
import { t } from "../i18n";
import type { Lang } from "../routes";
import { Seo } from "../seo";

export function NotFound({ lang }: { lang: Lang }) {
  const d = t(lang).notFound;
  return (
    <section className="notfound">
      <Seo title={d.title} description={d.text} noindex locale={t(lang).locale} />
      <span className="code404" aria-hidden="true">
        404
      </span>
      <Split as="h1" text={d.title} className="title" reveal={false} />
      <p>{d.text}</p>
      <a className="btn" href={`/${lang}/`} data-magnetic>
        {d.back}
      </a>
    </section>
  );
}
