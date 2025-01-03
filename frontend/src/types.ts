import { z } from 'zod'

export const UserIdSchema = z.string()
export const TeamIdSchema = z.string()
export const TaskIdSchema = z.string()
export const TestIdSchema = z.string()

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
  testId: TestIdSchema,
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

export const ExecutionOutputSchema = z.discriminatedUnion('type', [
  z.object({ type: z.literal('Aborted') }).merge(AbortedExecutionSchema),
  z.object({ type: z.literal('Error') }).merge(InternalErrorSchema),
  z.object({ type: z.literal('Finished') }).merge(FinishedExecutionSchema),
  z.object({ type: z.literal('Timeout') }).merge(FinishedExecutionSchema),
])

export const FinishedTestSchema = z.object({
  testId: z.string(),
  output: ExecutionOutputSchema,
})

export const FinishedTaskInfoSchema = z.object({
  taskId: TaskIdSchema,
  start: z.number().transform((ms) => new Date(ms)),
  end: z.number().transform((ms) => new Date(ms)),
  teamId: TeamIdSchema,
  revisionId: z.string(),
})

export const FinishedCompilerTaskSchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('BuildFailed'),
    info: FinishedTaskInfoSchema,
    buildOutput: ExecutionOutputSchema,
  }),
  z.object({
    type: z.literal('RanTests'),
    info: FinishedTaskInfoSchema,
    buildOutput: FinishedExecutionSchema,
    tests: z.array(FinishedTestSchema),
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

export const ExecutionExitStatusSchema = z.union([
  z.literal('Aborted'),
  z.literal('Error'),
  z.literal('Finished'),
  z.literal('Timeout'),
])

export const FinishedTestSummarySchema = z.object({
  testId: TestIdSchema,
  output: ExecutionExitStatusSchema,
})

export const FinishedCompilerTaskSummarySchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('BuildFailed'),
    info: FinishedTaskInfoSchema,
    status: ExecutionExitStatusSchema
  }),
  z.object({
    type: z.literal('RanTests'),
    info: FinishedTaskInfoSchema,
    tests: z.array(FinishedTestSummarySchema),
  }),
])

export const TeamInfoSchema = z.object({
  team: TeamSchema,
  members: z.array(UserSchema)
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
export type FinishedCompilerTaskSummary = z.infer<typeof FinishedCompilerTaskSummarySchema>
export type FinishedTestSummary = z.infer<typeof FinishedTestSummarySchema>
export type ExecutionExitStatus = z.infer<typeof ExecutionExitStatusSchema>
export type FinishedTaskInfo = z.infer<typeof FinishedTaskInfoSchema>
export type TaskId = z.infer<typeof TaskIdSchema>
export type TestId = z.infer<typeof TestIdSchema>
export type TeamInfo = z.infer<typeof TeamInfoSchema>
