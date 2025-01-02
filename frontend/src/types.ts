import { z } from 'zod'

export const UserIdSchema = z.string()
export const TeamIdSchema = z.string()

export const UserSchema = z.object({
  id: UserIdSchema,
  displayName: z.string(),
  team: TeamIdSchema.nullable(),
})

export const TeamSchema = z.object({
  id: TeamIdSchema,
  displayName: z.string(),
})

export const ShowMyselfResponseSchema = z.object({
  user: UserSchema,
  team: TeamSchema,
})

export const RepoSchema = z.object({
  team: TeamIdSchema,
  url: z.string().url('The repo URL is invalid'),
  autoFetch: z.boolean(),
})

export const CompilerTestSchema = z.object({
  testId: z.string(),
  timeout: z.number().describe('duration in ms'),
  runCommand: z.array(z.string()),
  expectedOutput: z.string(),
})

export const CompilerTaskSchema = z.object({
  taskId: z.string(),
  image: z.string(),
  buildCommand: z.array(z.string()),
  buildTimeout: z.number().describe('duration in ms'),
  tests: z.array(CompilerTestSchema),
})

export const FinishedExecutionSchema = z.object({
  stdout: z.string(),
  stderr: z.string(),
  runtime: z.number().describe('duration in ms'),
  exitStatus: z.number().nullable(),
})

export const AbortedExecutionSchema = z.object({
  stdout: z.string(),
  stderr: z.string(),
  runtime: z.number().describe('duration in ms'),
})

export const InternalErrorSchema = z.object({
  message: z.string(),
  runtime: z.number().describe('duration in ms'),
})

export const ExecutionOutputSchema = z.union([
  z.object({ Aborted: AbortedExecutionSchema }),
  z.object({ Error: InternalErrorSchema }),
  z.object({ Finished: FinishedExecutionSchema }),
  z.object({ Timeout: FinishedExecutionSchema }),
])

export const FinishedTestSchema = z.object({
  testId: z.string(),
  output: ExecutionOutputSchema,
})

export const FinishedCompilerTaskSchema = z.union([
  z.object({
    BuildFailed: z.object({
      start: z.number().transform((ms) => new Date(ms)),
      build_output: ExecutionOutputSchema,
    }),
  }),
  z.object({
    RanTests: z.object({
      start: z.number().transform((ms) => new Date(ms)),
      buildOutput: FinishedExecutionSchema,
      tests: z.array(FinishedTestSchema),
    }),
  }),
])

export const RunnerIdSchema = z.string()

export const RunnerInfoSchema = z.object({
  id: RunnerIdSchema,
  info: z.string(),
  currentTask: z.string().nullable(),
})

export const RunnerUpdateSchema = z.union([
  z.literal('StartedBuild'),
  z.object({ FinishedBuild: FinishedExecutionSchema }),
  z.object({ StartedTest: z.string() }),
  z.object({ FinishedTest: FinishedTestSchema }),
  z.literal('Done'),
])

export const RunnerWorkResponseSchema = z.object({
  task: CompilerTaskSchema.nullable(),
  reset: z.boolean(),
})

export const RunnerRegisterResponseSchema = z.object({
  reset: z.boolean(),
})

export const PatchRepoSchema = z.object({
  repoUrl: z.string().url('invalid url'),
  autoFetch: z.boolean(),
})

export type CompilerTest = z.infer<typeof CompilerTestSchema>
export type CompilerTask = z.infer<typeof CompilerTaskSchema>
export type FinishedExecution = z.infer<typeof FinishedExecutionSchema>
export type AbortedExecution = z.infer<typeof AbortedExecutionSchema>
export type InternalError = z.infer<typeof InternalErrorSchema>
export type ExecutionOutput = z.infer<typeof ExecutionOutputSchema>
export type FinishedTest = z.infer<typeof FinishedTestSchema>
export type FinishedCompilerTask = z.infer<typeof FinishedCompilerTaskSchema>
export type RunnerId = z.infer<typeof RunnerIdSchema>
export type RunnerInfo = z.infer<typeof RunnerInfoSchema>
export type RunnerUpdate = z.infer<typeof RunnerUpdateSchema>
export type RunnerWorkResponse = z.infer<typeof RunnerWorkResponseSchema>
export type RunnerRegisterResponse = z.infer<typeof RunnerRegisterResponseSchema>
export type User = z.infer<typeof UserSchema>
export type UserId = z.infer<typeof UserIdSchema>
export type TeamId = z.infer<typeof TeamIdSchema>
export type Team = z.infer<typeof TeamSchema>
export type PatchRepo = z.infer<typeof PatchRepoSchema>
export type ShowMyselfResponse = z.infer<typeof ShowMyselfResponseSchema>
export type Repo = z.infer<typeof RepoSchema>
