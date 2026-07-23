export const meta = {
  name: 'ideation-lab',
  description: 'Generative ideation roundtable that alternates collaborative discussion with online research over several rounds, growing a shared idea board instead of filtering it, then synthesizes an idea landscape and a ralplan-ready brief',
  whenToUse: 'Open "let us explore / generate ideas for X" work where the goal is a rich, research-informed idea space discovered together — not a ranked decision or a plan. Hand the output to ralplan afterwards.',
  phases: [
    { title: 'Frame' },
    { title: 'Discuss' },
    { title: 'Research' },
    { title: 'Synthesize' },
  ],
}

// ---------- thinker lenses (generative, not eliminative) ----------
const THINKERS = [
  { name: 'first-principles', brief: 'Reason from fundamentals. Propose ideas that rebuild the problem from the ground up, and extend others by asking what they would look like if stripped to essentials.' },
  { name: 'cross-pollinator', brief: 'Import mechanisms from unrelated fields (games, biology, finance, aviation, physical products) and graft them onto the ideas on the board.' },
  { name: 'user-empath', brief: 'Speak for the human in the loop. Push ideas toward what would feel like relief, delight, or trust; extend others to be more humane.' },
  { name: 'futurist', brief: 'Assume 2-4 years of capability ahead. Escalate existing ideas to their more ambitious form and propose what becomes cheap/possible soon.' },
  { name: 'synthesist', brief: 'Combine. Find pairs of board ideas that are stronger fused, and name the hybrid. Your main job is yes-and, not net-new.' },
  { name: 'pragmatist', brief: 'Find the version of each promising idea that is buildable soon. Propose concrete shapes and smallest-valuable-slices, not objections.' },
  { name: 'provocateur', brief: 'Push past the obvious. Offer bold, weird, or extreme framings — then point at the usable kernel so the panel can build on it.' },
]

// ---------- config ----------
const topic =
  typeof args === 'string'
    ? args
    : (args && (args.topic || args.question || args.prompt)) || null
if (!topic) throw new Error('ideation-lab: args.topic (or a string arg) is required')
const cfg = (typeof args === 'object' && args) || {}

function clamp(n, lo, hi) {
  return Math.max(lo, Math.min(hi, Math.floor(n)))
}
const DISCUSS_ROUNDS = clamp(cfg.rounds ?? 3, 2, 5) // research happens between discuss rounds → DISCUSS_ROUNDS-1 research phases
const N_THINKERS = clamp(cfg.thinkers ?? 5, 2, THINKERS.length)
const N_RESEARCH = clamp(cfg.researchers ?? 4, 1, 8) // research questions chased per research round
const context = cfg.context || null // optional caller-supplied background (codebase facts, prior decisions)
const noResearch = cfg.research === false

// ---------- schemas ----------
const FRAME_SCHEMA = {
  type: 'object',
  required: ['brief', 'directions', 'seed_questions'],
  properties: {
    brief: { type: 'string', description: 'One inviting paragraph: what we are exploring and the spirit of it (generative, open).' },
    directions: { type: 'array', items: { type: 'string' }, description: '3-6 distinct directions worth exploring (not constraints — invitations).' },
    what_good_looks_like: { type: 'array', items: { type: 'string' } },
    seed_questions: { type: 'array', items: { type: 'string' }, description: 'Open questions to spark the first discussion + first research.' },
  },
}
const THINKER_SCHEMA = {
  type: 'object',
  required: ['new_ideas', 'builds', 'research_requests'],
  properties: {
    new_ideas: {
      type: 'array',
      items: {
        type: 'object',
        required: ['title', 'summary'],
        properties: {
          title: { type: 'string' },
          summary: { type: 'string', description: '2-4 sentences: the idea and why it is interesting' },
          mechanism: { type: 'string', description: 'How it works / the core move' },
        },
      },
    },
    builds: {
      type: 'array',
      description: 'Yes-and extensions or fusions of ideas already on the board',
      items: {
        type: 'object',
        required: ['on', 'extension'],
        properties: {
          on: { type: 'string', description: 'Title(s) of the board idea(s) being extended/fused' },
          extension: { type: 'string', description: 'How it grows or what the fusion becomes' },
        },
      },
    },
    research_requests: { type: 'array', items: { type: 'string' }, description: 'What we should learn online to push these ideas further (prior art, examples, feasibility)' },
  },
}
const CURATOR_SCHEMA = {
  type: 'object',
  required: ['board', 'questions'],
  properties: {
    board: {
      type: 'array',
      description: 'The integrated idea board after this round — merged, de-duplicated, lineage preserved',
      items: {
        type: 'object',
        required: ['title', 'summary'],
        properties: {
          title: { type: 'string' },
          summary: { type: 'string' },
          lineage: { type: 'string', description: 'Where it came from / what it builds on (optional)' },
          promise_note: { type: 'string', description: 'One line on why it is promising or what is unresolved (optional)' },
        },
      },
    },
    questions: { type: 'array', items: { type: 'string' }, description: 'The most valuable open questions to research next, deduped and ranked' },
    round_note: { type: 'string', description: 'One-line read of where the thinking moved this round' },
  },
}
const RESEARCH_SCHEMA = {
  type: 'object',
  required: ['question', 'summary'],
  properties: {
    question: { type: 'string' },
    summary: { type: 'string', description: 'What the research found, in 3-6 sentences' },
    prior_art: {
      type: 'array',
      items: {
        type: 'object',
        required: ['name', 'what'],
        properties: {
          name: { type: 'string' },
          what: { type: 'string', description: 'What it is and the relevant idea/mechanism to steal or avoid' },
          link: { type: 'string' },
        },
      },
    },
    implications: { type: 'string', description: 'What this means for our ideas — what to push, drop, or add' },
    sources: { type: 'array', items: { type: 'string' } },
  },
}
const SYNTH_SCHEMA = {
  type: 'object',
  required: ['themes', 'most_promising', 'ralplan_brief'],
  properties: {
    themes: {
      type: 'array',
      items: {
        type: 'object',
        required: ['name', 'idea_titles', 'insight'],
        properties: {
          name: { type: 'string' },
          idea_titles: { type: 'array', items: { type: 'string' } },
          insight: { type: 'string' },
        },
      },
    },
    most_promising: {
      type: 'array',
      items: {
        type: 'object',
        required: ['title', 'why'],
        properties: {
          title: { type: 'string' },
          why: { type: 'string' },
          research_backing: { type: 'string', description: 'What the online research said about it' },
        },
      },
    },
    most_novel: {
      type: 'array',
      items: { type: 'object', required: ['title', 'why'], properties: { title: { type: 'string' }, why: { type: 'string' } } },
    },
    tensions: { type: 'array', items: { type: 'string' }, description: 'Real trade-offs / forks the ideas surface' },
    decisions_needed: { type: 'array', items: { type: 'string' }, description: 'What the user must decide before planning' },
    ralplan_brief: { type: 'string', description: 'A self-contained brief the user can hand to ralplan: the chosen direction space, the key ideas, the constraints, and the open decisions. Markdown.' },
  },
}

// ---------- helpers ----------
const j = (x, n = 7000) => JSON.stringify(x).slice(0, n)
function ideaKey(i) {
  return String(i.title || '').toLowerCase().replace(/[^a-z0-9]+/g, ' ').trim()
}

// ========================= PHASE 1: FRAME =========================
phase('Frame')
const frame = await agent(
  `You are opening a generative ideation lab. The goal is to EXPLORE and GENERATE ideas together, not to plan or filter.\n` +
    `Topic: ${topic}\n` +
    (context ? `Background the panel should know: ${j(context, 4000)}\n` : '') +
    `Write an inviting brief, list 3-6 distinct directions worth exploring (invitations, not walls), say what a good idea looks like here, and give seed questions to spark the first discussion and first research. Stay open — do not converge yet.`,
  { label: 'frame', phase: 'Frame', schema: FRAME_SCHEMA },
)
log(`Framed. Directions: ${(frame.directions || []).length}, seed questions: ${(frame.seed_questions || []).length}`)

// ---------- shared evolving state ----------
let board = [] // [{title, summary, lineage, promise_note}]
let questions = frame.seed_questions || []
let researchDigest = [] // [{question, summary, prior_art, implications}]
const chosen = THINKERS.slice(0, N_THINKERS)
const roundNotes = []

// ========================= DISCUSS / RESEARCH LOOP =========================
for (let round = 1; round <= DISCUSS_ROUNDS; round++) {
  // ----- DISCUSS -----
  phase('Discuss')
  const isFirst = round === 1
  const contributions = await parallel(
    chosen.map((t) => () =>
      agent(
        `You are the "${t.name}" thinker at a collaborative ideation roundtable (round ${round}/${DISCUSS_ROUNDS}).\n` +
          `Spirit: YES-AND. Build on what is on the board, combine ideas, push them further, and add new ones. Do NOT criticize or filter — that is a later, separate job.\n` +
          `Your lens: ${t.brief}\n\n` +
          `Frame (JSON): ${j(frame, 4000)}\n` +
          `Current idea board (JSON): ${board.length ? j(board) : '(empty — this is the first round, generate freely)'}\n` +
          `Latest research digest (JSON): ${researchDigest.length ? j(researchDigest) : '(none yet)'}\n` +
          `Open questions in play: ${j(questions, 2000)}\n\n` +
          (isFirst
            ? `Generate 2-4 bold new ideas from your lens. `
            : `Mostly BUILD: extend or fuse existing board ideas using the new research, and add 1-2 genuinely new ideas the research unlocked. `) +
          `Also list research_requests: what we should learn online next to push the best ideas further.`,
        { label: `discuss-r${round}:${t.name}`, phase: 'Discuss', schema: THINKER_SCHEMA },
      ),
    ),
  )
  const valid = contributions.filter(Boolean)

  // ----- WEAVE (curator integrates the round into the board) -----
  const curated = await agent(
    `You are the curator of a collaborative ideation roundtable, integrating round ${round}.\n` +
      `Frame (JSON): ${j(frame, 3000)}\n` +
      `Board BEFORE this round (JSON): ${board.length ? j(board) : '(empty)'}\n` +
      `This round's contributions from the panel (JSON): ${j(valid, 14000)}\n` +
      `Latest research digest (JSON): ${researchDigest.length ? j(researchDigest, 4000) : '(none)'}\n\n` +
      `Merge everything into one coherent idea board: fold builds into the ideas they extend, fuse near-duplicates (preserve lineage), keep distinct ideas distinct, and DO NOT drop ideas just because they are early — this is generative. ` +
      `Then output the most valuable open questions to research next (deduped, ranked, most idea-unlocking first), and a one-line note on where the thinking moved.`,
    { label: `weave-r${round}`, phase: 'Discuss', schema: CURATOR_SCHEMA },
  )
  board = curated.board || board
  questions = curated.questions || questions
  if (curated.round_note) roundNotes.push(`R${round}: ${curated.round_note}`)
  log(`Round ${round} discuss: board=${board.length} ideas, ${questions.length} open questions${curated.round_note ? ' — ' + curated.round_note : ''}`)

  // ----- RESEARCH (between discuss rounds, not after the last) -----
  if (!noResearch && round < DISCUSS_ROUNDS) {
    phase('Research')
    const toChase = (questions || []).slice(0, N_RESEARCH)
    if (toChase.length) {
      const findings = await parallel(
        toChase.map((q, i) => () =>
          agent(
            `You are a research agent for an ideation lab. Investigate this question with ONLINE research.\n` +
              `Use your available web search and fetch tools (search the web, then read the most relevant sources) — load them via tool search if needed. Prefer concrete prior art, real products, papers, and patterns over generic advice.\n\n` +
              `Topic context: ${topic}\n` +
              `Question (focus #${i + 1}): ${q}\n` +
              `Ideas currently in play (titles): ${j((board || []).map((b) => b.title), 2000)}\n\n` +
              `Return a tight finding: what you learned, concrete prior art (name + what to steal/avoid + link), and the implications for our ideas (what to push, add, or reshape). Cite sources.`,
            { label: `research-r${round}:q${i + 1}`, phase: 'Research', model: 'sonnet', schema: RESEARCH_SCHEMA },
          ),
        ),
      )
      researchDigest = findings.filter(Boolean)
      log(`Round ${round} research: ${researchDigest.length} questions investigated online`)
    }
  }
}

// ========================= SYNTHESIZE =========================
phase('Synthesize')
const synthesis = await agent(
  `You are closing a generative ideation lab. Curate the final landscape — do not invent a plan, and do not throw ideas away; organize and illuminate them.\n` +
    `Frame (JSON): ${j(frame, 3000)}\n` +
    `Final idea board (JSON): ${j(board, 16000)}\n` +
    `All research findings (JSON): ${j(researchDigest, 8000)}\n` +
    `Round notes: ${j(roundNotes, 1500)}\n\n` +
    `Produce: themes (clusters of ideas + the insight each reveals); the most promising ideas with why + what research backed them; the most novel/surprising ideas; the real tensions/forks the space surfaces; the decisions the user must make before planning; ` +
    `and finally a self-contained markdown ralplan_brief the user can hand straight to ralplan (chosen direction space, key ideas, constraints, open decisions). Keep the spirit generative — this is a map of a rich idea space, not a verdict.`,
  { label: 'synthesize', phase: 'Synthesize', schema: SYNTH_SCHEMA },
)

return {
  topic,
  frame,
  rounds: { discuss: DISCUSS_ROUNDS, research: noResearch ? 0 : DISCUSS_ROUNDS - 1, thinkers: chosen.length },
  round_notes: roundNotes,
  board,
  research: researchDigest,
  themes: synthesis.themes || [],
  most_promising: synthesis.most_promising || [],
  most_novel: synthesis.most_novel || [],
  tensions: synthesis.tensions || [],
  decisions_needed: synthesis.decisions_needed || [],
  ralplan_brief: synthesis.ralplan_brief || '',
}
