import { createFileRoute, Link } from '@tanstack/react-router'
import { HomeLayout } from 'fumadocs-ui/layouts/home'
import BrandMark from '@/components/BrandMark'
import Wordmark from '@/components/Wordmark'
import { baseOptions } from '@/lib/layout.shared'

export const Route = createFileRoute('/')({ component: Home })

const taskCards = [
  {
    number: '01',
    title: 'Start streaming',
    description:
      'Go from a fresh machine to your first stream with the shortest setup path.',
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
        <section className="border-b border-fd-border">
          <div className="mx-auto grid w-full max-w-6xl gap-12 px-6 py-16 md:grid-cols-[minmax(0,1.1fr)_minmax(18rem,0.9fr)] md:px-8 md:py-24">
            <div className="flex flex-col justify-center">
              <div className="mb-7 flex items-center gap-3 text-xs font-semibold uppercase tracking-[0.18em] text-fd-muted-foreground">
                <BrandMark className="size-8 rounded-lg shadow-[0_8px_30px_rgba(8,145,178,0.28)]" />
                <span>Slipstream documentation</span>
              </div>
              <h1 className="max-w-3xl text-balance text-4xl font-semibold tracking-tight text-fd-foreground md:text-6xl">
                Put your desktop on the screen in front of you.
              </h1>
              <p className="mt-6 max-w-2xl text-lg leading-8 text-fd-muted-foreground">
                Install a streaming host, pair a device, and keep everything in reach from
                Slipstream&apos;s browser console. The guides follow the same path you will take
                on a real machine.
              </p>
              <div className="mt-8 flex flex-wrap gap-3">
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

            <div className="flex items-center">
              <div className="w-full rounded-2xl border border-fd-border bg-fd-card p-5 shadow-[0_18px_60px_rgba(8,145,178,0.12)] sm:p-6">
                <div className="flex items-start justify-between gap-4">
                  <div className="flex items-center gap-3">
                    <BrandMark className="size-10 rounded-xl" />
                    <div>
                      <Wordmark className="h-4" />
                      <p className="mt-1 text-sm text-fd-muted-foreground">
                        Streaming host and browser console
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
                      Capture, encode, and serve your desktop.
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
                Slipstream is built for a LAN or VPN. Read the security guide before exposing a
                host beyond devices you trust.
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
