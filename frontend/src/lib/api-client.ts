import createClient from "openapi-fetch";
import type { paths } from "./api-schema.ts";

export const client = createClient<paths>();
