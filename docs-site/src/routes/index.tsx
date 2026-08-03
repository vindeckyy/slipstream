import { createFileRoute, Link } from '@tanstack/react-router'
import { HomeLayout } from 'fumadocs-ui/layouts/home'
import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'
import { baseOptions } from '@/lib/layout.shared'

export const Route = createFileRoute('/')({ component: Home })

const audiences = [
  {
    eyebrow: 'Play',
    title: 'Games on the screen in front of you',
    description:
      'Launch titles from your host library on a TV, Steam Deck, phone, or couch PC. Native Slipstream apps, plus Moonlight when you want GameStream.',
    links: [
      { label: 'Quick Start', slug: 'quickstart' },
      { label: 'Game library', slug: 'game-library' },
      { label: 'Moonlight', slug: 'moonlight' },
    ],
  },
  {
    eyebrow: 'Work',
    title: 'Your real desktop while you are at the office',
    description:
      'Reach the machine on your desk from a work laptop over LAN or VPN. Full desktop, absolute mouse, clipboard, and Workstation or Hot-desk display presets.',
    links: [
      { label: 'Quick Start', slug: 'quickstart' },
      { label: 'Virtual displays', slug: 'virtual-displays' },
      { label: 'Input', slug: 'input' },
      { label: 'Clipboard', slug: 'clipboard' },
    ],
  },
] as const

const taskCards = [
  {
    number: '01',
    title: 'Start streaming',
    description:
      'Go from a fresh machine to your first desktop or game stream with the shortest setup path.',
    slug: 'quickstart',
    action: 'Open Quick Start',
  },
  {
    number: '02',
    title: 'Install the host',
    description:
      'Choose the guide for Ubuntu, Fedora, Arch, SteamOS, or Windows.',
    slug: 'install',
    action: 'Choose an install guide',
  },
  {
    number: '03',
    title: 'Connect a device',
    description:
      'Pick a native Slipstream app or connect any device that runs Moonlight.',
    slug: 'clients',
    action: 'Choose a client',
  },
  {
    number: '04',
    title: 'Open the browser console',
    description:
      'Pair devices, check host status, manage displays, and keep the host current.',
    slug: 'web-console',
    action: 'Open the console guide',
  },
] as const

const referenceLinks = [
  {
    title: 'Check compatibility',
    description: 'Compare host platforms, GPUs, encoders, and client features.',
    slug: 'support-matrix',
  },
  {
    title: 'Fix a rough edge',
    description: 'Start from the symptom and work through the troubleshooting guide.',
    slug: 'troubleshooting',
  },
] as const

function Home() {
  return (
    <HomeLayout {...baseOptions()}>
      <main className="flex flex-1 flex-col">
        <section className="relative overflow-hidden border-b border-fd-border">
          <div
            aria-hidden="true"
            className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_top_left,color-mix(in_oklab,var(--ss-brand)_18%,transparent),transparent_55%),radial-gradient(ellipse_at_bottom_right,color-mix(in_oklab,var(--ss-brand-light)_12%,transparent),transparent_50%)]"
          />
          <div className="relative mx-auto grid w-full max-w-6xl gap-12 px-6 py-16 md:grid-cols-[minmax(0,1.1fr)_minmax(18rem,0.9fr)] md:px-8 md:py-24">
            <div className="flex flex-col justify-center">
              <div className="mb-8 flex items-center gap-4">
                <BrandMark className="size-12 rounded-xl shadow-[0_8px_30px_rgba(8,145,178,0.28)] motion-safe:animate-[ss-rise_700ms_ease-out]" />
                <Wordmark className="h-7 motion-safe:animate-[ss-rise_700ms_ease-out]" />
              </div>
              <h1 className="max-w-3xl text-balance text-4xl font-semibold tracking-tight text-fd-foreground motion-safe:animate-[ss-rise_800ms_ease-out] md:text-5xl lg:text-6xl">
                Play from the couch. Work from the office. Same desktop.
              </h1>
              <p className="mt-6 max-w-2xl text-lg leading-8 text-fd-muted-foreground motion-safe:animate-[ss-rise_900ms_ease-out]">
                Slipstream is a private host for low-latency desktop and game streaming. Install
                once, pair a device, and use your real machine for games at home or focused work
                away from your desk — no cloud, no accounts.
              </p>
              <div className="mt-8 flex flex-wrap gap-3 motion-safe:animate-[ss-rise_1s_ease-out]">
                <Link
                  to="/docs/$"
                  params={{ _splat: 'quickstart' }}
                  className="rounded-lg bg-brand px-5 py-3 font-medium text-white shadow-sm transition-colors hover:bg-brand/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
                >
                  Start with Quick Start
                </Link>
                <Link
                  to="/docs/$"
                  params={{ _splat: '' }}
                  className="rounded-lg border border-fd-border bg-fd-card px-5 py-3 font-medium text-fd-foreground transition-colors hover:border-fd-primary hover:text-fd-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
                >
                  Browse all docs
                </Link>
              </div>
              <p className="mt-5 text-sm text-fd-muted-foreground">
                Native apps for every major platform, plus Moonlight compatibility when you need
                it.
              </p>
            </div>

            <div className="flex items-center motion-safe:animate-[ss-rise_900ms_ease-out]">
              <div className="w-full rounded-2xl border border-fd-border bg-fd-card/90 p-5 shadow-[0_18px_60px_rgba(8,145,178,0.12)] backdrop-blur-sm sm:p-6">
                <div className="flex items-start justify-between gap-4">
                  <div className="flex items-center gap-3">
                    <BrandMark className="size-10 rounded-xl" />
                    <div>
                      <Wordmark className="h-4" />
                      <p className="mt-1 text-sm text-fd-muted-foreground">
                        Desktop and game streaming host
                      </p>
                    </div>
                  </div>
                  <span className="rounded-full bg-fd-primary/10 px-2.5 py-1 text-xs font-medium text-fd-primary">
                    Local network
                  </span>
                </div>
                <div className="mt-7 grid gap-3 sm:grid-cols-2">
                  <div className="rounded-xl border border-fd-border bg-fd-background p-4">
                    <p className="text-xs font-semibold uppercase tracking-[0.16em] text-fd-muted-foreground">
                      Host
                    </p>
                    <p className="mt-2 font-medium text-fd-foreground">Linux or Windows</p>
                    <p className="mt-1 text-sm text-fd-muted-foreground">
                      Capture, encode, and serve your desktop or games.
                    </p>
                  </div>
                  <div className="rounded-xl border border-fd-border bg-fd-background p-4">
                    <p className="text-xs font-semibold uppercase tracking-[0.16em] text-fd-muted-foreground">
                      Client
                    </p>
                    <p className="mt-2 font-medium text-fd-foreground">Any screen you use</p>
                    <p className="mt-1 text-sm text-fd-muted-foreground">
                      Native Slipstream apps or Moonlight.
                    </p>
                  </div>
                </div>
                <div className="mt-5 flex items-center gap-2 border-t border-fd-border pt-4 text-sm text-fd-muted-foreground">
                  <span aria-hidden="true" className="size-2 rounded-full bg-emerald-400" />
                  <span>Pair once, then reconnect with a pinned identity.</span>
                </div>
              </div>
            </div>
          </div>
        </section>

        <section className="border-b border-fd-border bg-fd-card/40">
          <div className="mx-auto w-full max-w-6xl px-6 py-16 md:px-8 md:py-20">
            <div className="max-w-2xl">
              <p className="text-xs font-semibold uppercase tracking-[0.18em] text-fd-primary">
                Who it is for
              </p>
              <h2 className="mt-3 text-3xl font-semibold tracking-tight text-fd-foreground md:text-4xl">
                Built for gamers and for people who need their desktop at work.
              </h2>
              <p className="mt-4 text-base leading-7 text-fd-muted-foreground">
                Same private host. Two jobs people actually hire it for.
              </p>
            </div>

            <div className="mt-10 grid gap-6 md:grid-cols-2">
              {audiences.map((audience) => (
                <div
                  key={audience.eyebrow}
                  className="rounded-2xl border border-fd-border bg-fd-background p-6 transition-transform duration-300 motion-safe:hover:-translate-y-0.5 md:p-8"
                >
                  <p className="text-xs font-semibold uppercase tracking-[0.18em] text-fd-primary">
                    {audience.eyebrow}
                  </p>
                  <h3 className="mt-4 text-2xl font-semibold tracking-tight text-fd-foreground">
                    {audience.title}
                  </h3>
                  <p className="mt-3 max-w-md leading-7 text-fd-muted-foreground">
                    {audience.description}
                  </p>
                  <div className="mt-6 flex flex-wrap gap-x-4 gap-y-2">
                    {audience.links.map((link) => (
                      <Link
                        key={link.slug}
                        to="/docs/$"
                        params={{ _splat: link.slug }}
                        className="text-sm font-medium text-fd-primary underline-offset-4 transition-colors hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
                      >
                        {link.label}
                      </Link>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>

        <section className="mx-auto w-full max-w-6xl px-6 py-16 md:px-8 md:py-20">
          <div className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.18em] text-fd-primary">
                Choose a task
              </p>
              <h2 className="mt-3 text-3xl font-semibold tracking-tight text-fd-foreground md:text-4xl">
                The shortest route to a working stream.
              </h2>
            </div>
            <p className="max-w-md text-sm leading-6 text-fd-muted-foreground md:text-right">
              Start with the job in front of you. Each guide links to the deeper reference when
              you need it.
            </p>
          </div>

          <div className="mt-10 grid gap-4 md:grid-cols-2">
            {taskCards.map((task) => (
              <Link
                key={task.slug}
                to="/docs/$"
                params={{ _splat: task.slug }}
                className="group rounded-2xl border border-fd-border bg-fd-card p-6 transition-colors hover:border-fd-primary hover:bg-fd-primary/5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
              >
                <div className="flex items-start justify-between gap-4">
                  <span className="text-xs font-semibold tracking-[0.18em] text-fd-primary">
                    {task.number}
                  </span>
                  <span className="text-xs text-fd-muted-foreground group-hover:text-fd-primary">
                    {task.action}
                  </span>
                </div>
                <h3 className="mt-10 text-xl font-semibold text-fd-foreground">{task.title}</h3>
                <p className="mt-2 max-w-md leading-7 text-fd-muted-foreground">
                  {task.description}
                </p>
              </Link>
            ))}
          </div>
        </section>

        <section className="border-y border-fd-border bg-fd-card/40">
          <div className="mx-auto w-full max-w-6xl px-6 py-14 md:px-8 md:py-16">
            <div className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
              <div>
                <p className="text-xs font-semibold uppercase tracking-[0.18em] text-fd-primary">
                  Keep moving
                </p>
                <h2 className="mt-3 text-2xl font-semibold tracking-tight text-fd-foreground md:text-3xl">
                  Reference for the parts that need a closer look.
                </h2>
              </div>
              <p className="max-w-md text-sm leading-6 text-fd-muted-foreground md:text-right">
                Compatibility, recovery, and automation live here once the first stream is working.
              </p>
            </div>

            <div className="mt-8 grid gap-4 md:grid-cols-3">
              {referenceLinks.map((item) => (
                <Link
                  key={item.slug}
                  to="/docs/$"
                  params={{ _splat: item.slug }}
                  className="rounded-xl border border-fd-border bg-fd-background p-5 transition-colors hover:border-fd-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
                >
                  <h3 className="font-semibold text-fd-foreground">{item.title}</h3>
                  <p className="mt-2 text-sm leading-6 text-fd-muted-foreground">
                    {item.description}
                  </p>
                </Link>
              ))}
              <a
                href="/api"
                className="rounded-xl border border-fd-border bg-fd-background p-5 transition-colors hover:border-fd-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
              >
                <h3 className="font-semibold text-fd-foreground">Build against the host</h3>
                <p className="mt-2 text-sm leading-6 text-fd-muted-foreground">
                  Use the interactive management API reference for status, pairing, and library
                  control.
                </p>
              </a>
            </div>
          </div>
        </section>

        <section className="mx-auto w-full max-w-6xl px-6 py-14 md:px-8 md:py-16">
          <div className="flex flex-col gap-5 rounded-2xl border border-fd-border bg-fd-primary/5 p-6 md:flex-row md:items-center md:justify-between md:p-8">
            <div>
              <h2 className="text-xl font-semibold text-fd-foreground">Keep the host on a trusted network.</h2>
              <p className="mt-2 max-w-2xl text-sm leading-6 text-fd-muted-foreground">
                Slipstream is built for a LAN or VPN — including the path from an office laptop back
                to a home desktop. Read the security guide before exposing a host beyond devices you
                trust.
              </p>
            </div>
            <Link
              to="/docs/$"
              params={{ _splat: 'security' }}
              className="shrink-0 rounded-lg border border-fd-border bg-fd-background px-4 py-2.5 text-sm font-medium text-fd-foreground transition-colors hover:border-fd-primary hover:text-fd-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-fd-primary"
            >
              Read Security &amp; Safe Use
            </Link>
          </div>
        </section>
      </main>
    </HomeLayout>
  )
}
