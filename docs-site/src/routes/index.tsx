import { createFileRoute, Link } from '@tanstack/react-router'
import { HomeLayout } from 'fumadocs-ui/layouts/home'
import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'
import { baseOptions } from '@/lib/layout.shared'

export const Route = createFileRoute('/')({ component: Home })

const journeys = [
  {
    eyebrow: 'Play',
    title: 'Games on every screen you own',
    description:
      'Stream from a powerful Linux host to a TV, Steam Deck, iPhone, or Android device. Launch from the game library, use Capture mouse for shooters, and keep Moonlight when you want it.',
    href: 'play',
    cta: 'Open the Play guide',
    secondary: [
      { label: 'Game library', slug: 'game-library' },
      { label: 'Controllers', slug: 'controllers' },
      { label: 'HDR', slug: 'hdr' },
    ],
  },
  {
    eyebrow: 'Work',
    title: 'Your real desktop at the office',
    description:
      'Reach the machine you left at home over a trusted VPN. Absolute mouse, clipboard, Workstation or Hot-desk presets, and picture settings tuned for sharp text.',
    href: 'desktop-at-work',
    cta: 'Open Desktop at work',
    secondary: [
      { label: 'Network & VPN', slug: 'network-and-vpn' },
      { label: 'Picture quality', slug: 'picture-quality' },
      { label: 'Clipboard', slug: 'clipboard' },
    ],
  },
] as const

const steps = [
  {
    number: '01',
    title: 'Install the host',
    description: 'Linux packages for Ubuntu, Fedora, Arch, Bazzite, SteamOS, or NixOS.',
    slug: 'install',
  },
  {
    number: '02',
    title: 'Pair a client',
    description: 'iPhone, Android, Steam Deck, or Moonlight when you enable GameStream.',
    slug: 'pairing',
  },
  {
    number: '03',
    title: 'Tune the stream',
    description: 'Displays, mouse mode, bitrate, HDR, clipboard, and Work vs Play profiles.',
    slug: 'client-settings',
  },
] as const

const guides = [
  {
    title: 'How it works',
    description: 'Virtual displays, capture to decode, native vs GameStream, pairing and discovery.',
    slug: 'how-it-works',
  },
  {
    title: 'Security',
    description: 'Trusted LAN or VPN only. Pairing is the boundary. Never port-forward.',
    slug: 'security',
  },
  {
    title: 'Support matrix',
    description: 'Host desktops, GPUs, encoders, and client features - read from the code.',
    slug: 'support-matrix',
  },
  {
    title: 'Web console',
    description: 'Pair devices, manage displays, library, plugins, and live host status.',
    slug: 'web-console',
  },
  {
    title: 'Troubleshooting',
    description: 'Start from the symptom: discovery, black screen, input, stutter, Office / VPN.',
    slug: 'troubleshooting',
  },
  {
    title: 'API reference',
    description: 'Interactive OpenAPI for status, pairing, library, and host control.',
    href: '/api',
  },
] as const

function Home() {
  return (
    <HomeLayout {...baseOptions()}>
      <main className="flex flex-1 flex-col">
        <section className="relative overflow-hidden border-b border-fd-border">
          <div
            aria-hidden="true"
            className="pointer-events-none absolute inset-0 ss-hero-glow"
          />
          <div
            aria-hidden="true"
            className="pointer-events-none absolute inset-0 ss-grid-fade opacity-[0.35] dark:opacity-[0.22]"
          />
          <div className="relative mx-auto flex w-full max-w-6xl flex-col px-6 py-20 md:px-8 md:py-28">
            <div className="mb-10 flex items-center gap-4 motion-safe:animate-[ss-rise_700ms_ease-out]">
              <BrandMark className="size-14 rounded-2xl shadow-[0_12px_40px_rgba(8,145,178,0.32)]" />
              <div>
                <Wordmark className="h-8 md:h-9" />
                <p className="mt-2 text-xs font-semibold uppercase tracking-[0.2em] text-fd-muted-foreground">
                  Documentation
                </p>
              </div>
            </div>

            <h1 className="max-w-4xl text-balance text-4xl font-semibold tracking-tight text-fd-foreground motion-safe:animate-[ss-rise_800ms_ease-out] md:text-6xl lg:text-[4.25rem] lg:leading-[1.05]">
              Your desktop and your games, documented end to end.
            </h1>
            <p className="mt-7 max-w-2xl text-lg leading-8 text-fd-muted-foreground motion-safe:animate-[ss-rise_900ms_ease-out] md:text-xl md:leading-9">
              Slipstream is a private host for low-latency desktop and game streaming. These guides
              take you from install to a work session or a couch stream on a LAN or a VPN you
              trust.
            </p>

            <div className="mt-10 flex flex-wrap gap-3 motion-safe:animate-[ss-rise_1s_ease-out]">
              <Link
                to="/docs/$"
                params={{ _splat: 'quickstart' }}
                className="rounded-lg bg-brand px-6 py-3.5 font-medium text-white shadow-sm transition-[transform,background-color] duration-200 hover:bg-brand/90 motion-safe:hover:-translate-y-0.5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
              >
                Quick Start
              </Link>
              <Link
                to="/docs/$"
                params={{ _splat: 'play' }}
                className="rounded-lg border border-fd-border bg-fd-card/80 px-6 py-3.5 font-medium text-fd-foreground backdrop-blur-sm transition-[transform,border-color,color] duration-200 hover:border-fd-primary hover:text-fd-primary motion-safe:hover:-translate-y-0.5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
              >
                Play guide
              </Link>
              <Link
                to="/docs/$"
                params={{ _splat: 'desktop-at-work' }}
                className="rounded-lg border border-fd-border bg-fd-card/80 px-6 py-3.5 font-medium text-fd-foreground backdrop-blur-sm transition-[transform,border-color,color] duration-200 hover:border-fd-primary hover:text-fd-primary motion-safe:hover:-translate-y-0.5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
              >
                Work guide
              </Link>
            </div>

            <p className="mt-6 text-sm text-fd-muted-foreground motion-safe:animate-[ss-rise_1.05s_ease-out]">
              Native clients for every major platform · Moonlight-compatible when you enable GameStream ·
              No accounts, no cloud
            </p>
          </div>
        </section>

        <section className="border-b border-fd-border">
          <div className="mx-auto grid w-full max-w-6xl gap-0 md:grid-cols-2">
            {journeys.map((journey, index) => (
              <div
                key={journey.eyebrow}
                className={`flex flex-col border-fd-border bg-fd-card/30 px-6 py-14 md:px-8 md:py-16 ${
                  index === 0 ? 'md:border-r' : ''
                }`}
              >
                <p className="text-xs font-semibold uppercase tracking-[0.2em] text-fd-primary">
                  {journey.eyebrow}
                </p>
                <h2 className="mt-4 max-w-md text-3xl font-semibold tracking-tight text-fd-foreground md:text-4xl">
                  {journey.title}
                </h2>
                <p className="mt-4 max-w-md text-base leading-7 text-fd-muted-foreground">
                  {journey.description}
                </p>
                <div className="mt-8 flex flex-wrap items-center gap-x-5 gap-y-3">
                  <Link
                    to="/docs/$"
                    params={{ _splat: journey.href }}
                    className="font-medium text-fd-primary underline-offset-4 transition-colors hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
                  >
                    {journey.cta}
                  </Link>
                  {journey.secondary.map((link) => (
                    <Link
                      key={link.slug}
                      to="/docs/$"
                      params={{ _splat: link.slug }}
                      className="text-sm text-fd-muted-foreground underline-offset-4 transition-colors hover:text-fd-primary hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
                    >
                      {link.label}
                    </Link>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="mx-auto w-full max-w-6xl px-6 py-16 md:px-8 md:py-20">
          <div className="max-w-2xl">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-fd-primary">
              First stream
            </p>
            <h2 className="mt-3 text-3xl font-semibold tracking-tight text-fd-foreground md:text-4xl">
              Three steps. Then pick Play or Work.
            </h2>
            <p className="mt-4 text-base leading-7 text-fd-muted-foreground">
              The shared path is short. Audience-specific tuning comes after the picture is up.
            </p>
          </div>

          <ol className="mt-12 grid gap-8 md:grid-cols-3 md:gap-6">
            {steps.map((step) => (
              <li key={step.slug} className="relative">
                <Link
                  to="/docs/$"
                  params={{ _splat: step.slug }}
                  className="group block focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-fd-primary"
                >
                  <span className="text-xs font-semibold tracking-[0.2em] text-fd-primary">
                    {step.number}
                  </span>
                  <h3 className="mt-4 text-xl font-semibold text-fd-foreground transition-colors group-hover:text-fd-primary">
                    {step.title}
                  </h3>
                  <p className="mt-2 text-sm leading-6 text-fd-muted-foreground">
                    {step.description}
                  </p>
                </Link>
              </li>
            ))}
          </ol>

          <div className="mt-10">
            <Link
              to="/docs/$"
              params={{ _splat: 'quickstart' }}
              className="inline-flex rounded-lg border border-fd-border bg-fd-card px-5 py-3 text-sm font-medium text-fd-foreground transition-colors hover:border-fd-primary hover:text-fd-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
            >
              Open the full Quick Start
            </Link>
          </div>
        </section>

        <section className="border-y border-fd-border bg-fd-card/40">
          <div className="mx-auto w-full max-w-6xl px-6 py-16 md:px-8 md:py-20">
            <div className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
              <div>
                <p className="text-xs font-semibold uppercase tracking-[0.2em] text-fd-primary">
                  Guides
                </p>
                <h2 className="mt-3 text-3xl font-semibold tracking-tight text-fd-foreground md:text-4xl">
                  Everything else, by job.
                </h2>
              </div>
              <p className="max-w-md text-sm leading-6 text-fd-muted-foreground md:text-right">
                Architecture, security, compatibility, the browser console, recovery, and the API.
              </p>
            </div>

            <div className="mt-10 grid gap-x-8 gap-y-8 sm:grid-cols-2 lg:grid-cols-3">
              {guides.map((guide) =>
                'href' in guide && guide.href ? (
                  <a
                    key={guide.title}
                    href={guide.href}
                    className="group block border-t border-fd-border pt-5 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-fd-primary"
                  >
                    <h3 className="font-semibold text-fd-foreground transition-colors group-hover:text-fd-primary">
                      {guide.title}
                    </h3>
                    <p className="mt-2 text-sm leading-6 text-fd-muted-foreground">
                      {guide.description}
                    </p>
                  </a>
                ) : (
                  <Link
                    key={guide.title}
                    to="/docs/$"
                    params={{ _splat: guide.slug! }}
                    className="group block border-t border-fd-border pt-5 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-fd-primary"
                  >
                    <h3 className="font-semibold text-fd-foreground transition-colors group-hover:text-fd-primary">
                      {guide.title}
                    </h3>
                    <p className="mt-2 text-sm leading-6 text-fd-muted-foreground">
                      {guide.description}
                    </p>
                  </Link>
                ),
              )}
            </div>
          </div>
        </section>

        <section className="mx-auto w-full max-w-6xl px-6 py-16 md:px-8 md:py-20">
          <div className="relative overflow-hidden rounded-2xl border border-fd-border bg-fd-primary/5 px-6 py-10 md:px-10 md:py-12">
            <div aria-hidden="true" className="pointer-events-none absolute inset-0 ss-hero-glow opacity-60" />
            <div className="relative flex flex-col gap-6 md:flex-row md:items-center md:justify-between">
              <div className="max-w-2xl">
                <h2 className="text-2xl font-semibold tracking-tight text-fd-foreground md:text-3xl">
                  Keep the host on a trusted network.
                </h2>
                <p className="mt-3 text-base leading-7 text-fd-muted-foreground">
                  Slipstream is built for a LAN or a private VPN - including office laptop to home
                  desktop. Pairing is the security boundary. Do not port-forward to the public internet.
                </p>
              </div>
              <div className="flex shrink-0 flex-wrap gap-3">
                <Link
                  to="/docs/$"
                  params={{ _splat: 'security' }}
                  className="rounded-lg border border-fd-border bg-fd-background px-4 py-2.5 text-sm font-medium text-fd-foreground transition-colors hover:border-fd-primary hover:text-fd-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
                >
                  Security guide
                </Link>
                <Link
                  to="/docs/$"
                  params={{ _splat: 'network-and-vpn' }}
                  className="rounded-lg border border-fd-border bg-fd-background px-4 py-2.5 text-sm font-medium text-fd-foreground transition-colors hover:border-fd-primary hover:text-fd-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
                >
                  Network &amp; VPN
                </Link>
              </div>
            </div>
          </div>
        </section>
      </main>
    </HomeLayout>
  )
}
